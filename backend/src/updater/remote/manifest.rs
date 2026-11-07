//! 目标版本清单：字节级解析 + 摘要可信的本地缓存

use crate::updater::model::{ManifestDigest, UpdateManifest};
use crate::updater::remote::fetch;
use crate::updater::verify::verify_manifest;
use anyhow::{anyhow, Context};
use reqwest::Client;
use std::fs;
use std::path::Path;

/// 解析清单字节
pub fn parse_manifest(bytes: &[u8]) -> anyhow::Result<UpdateManifest> {
    serde_json::from_slice(bytes).context(anyhow!("清单格式不符合规范"))
}

/// 取目标版本清单字节：缓存优先，缓存不可信则删掉回源
pub async fn load(
    client: &Client,
    url: &str,
    cache_path: &Path,
    expected: Option<&ManifestDigest>,
) -> anyhow::Result<Vec<u8>> {
    if let Ok(cached) = fs::read(cache_path) {
        if is_trusted(&cached, expected).is_ok() {
            return Ok(cached);
        }
        // 自愈：解析不了或摘要不符，都说明这份缓存不该再被信任
        if let Err(error) = fs::remove_file(cache_path) {
            tracing::warn!("删除失效的更新清单缓存失败（{}）：{error}", cache_path.display());
        }
    }

    let fetched = fetch::fetch_bytes(client, url).await?;
    // 连 JSON 都不是 → 无可继续，直接暴露（与"摘要不符"区别对待）
    parse_manifest(&fetched)?;

    if let Some(digest) = expected {
        if let Err(error) = verify_manifest(&fetched, digest) {
            tracing::warn!(
                "更新清单与版本索引的摘要不符，本次使用但不写缓存（{url}，索引期望 {}）：{error}",
                digest.sha256
            );
            return Ok(fetched);
        }
    }

    if let Err(error) = write_atomic(cache_path, &fetched) {
        // 缓存只是加速手段，写不进去不影响本次检查
        tracing::warn!("写入更新清单缓存失败（{}）：{error}", cache_path.display());
    }
    Ok(fetched)
}

/// 缓存字节是否可直接使用：能解析 **且**（有摘要时）摘要匹配
fn is_trusted(bytes: &[u8], expected: Option<&ManifestDigest>) -> anyhow::Result<()> {
    parse_manifest(bytes)?;
    match expected {
        Some(digest) => verify_manifest(bytes, digest),
        None => Ok(()),
    }
}

/// 同卷原子写：先写 `*.tmp`，再 rename 覆盖
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    // 同目录 rename 在同卷内原子；Windows 上 std 的实现是 MoveFileEx + REPLACE_EXISTING
    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::app_paths;
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;
    use std::thread::JoinHandle;
    use std::time::Duration;
    use tiny_http::{Response, Server};

    /// 一份能被 `UpdateManifest` 解析的最小清单
    const BODY: &[u8] = br#"{"version":"0.8.6","releaseNotes":null,"platforms":{}}"#;

    /// 另一份"合法但不是目标版本"的清单，用来模拟陈旧缓存
    const OTHER: &[u8] = br#"{"version":"0.8.5","releaseNotes":null,"platforms":{}}"#;

    /// 轻量 mock 清单服务器：只回应一次
    struct MockServer {
        url: String,
        handle: JoinHandle<()>,
    }

    impl MockServer {
        fn spawn(body: Vec<u8>) -> Self {
            let server = Server::http("127.0.0.1:0").expect("mock 服务器启动失败");
            let port = server.server_addr().to_ip().expect("无法获取端口").port();
            let url = format!("http://127.0.0.1:{port}/latest-cnb.json");
            let handle = std::thread::spawn(move || {
                if let Ok(Some(request)) = server.recv_timeout(Duration::from_secs(10)) {
                    let _ = request.respond(Response::from_data(body));
                }
            });
            Self { url, handle }
        }
    }

    /// 每个用例独占一个缓存路径（进程内测试目录本身已按进程随机）
    fn cache_path(tag: &str) -> PathBuf {
        let path = app_paths::temp_dir().join(format!("update/manifest-cache-{tag}.json"));
        // 目录由各写入点自建（`fs::write` 不创建父目录），这里同样不能假设它已存在
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        path
    }

    fn digest_of(bytes: &[u8]) -> ManifestDigest {
        ManifestDigest {
            sha256: hex::encode(Sha256::digest(bytes)),
            signature: None,
        }
    }

    /// 不可达地址：一旦真发起请求就会立刻失败，用来证明"零网络"
    const UNREACHABLE: &str = "http://127.0.0.1:1/latest-cnb.json";

    #[test]
    fn cache_hit_with_matching_digest_skips_network() {
        tauri::async_runtime::block_on(async {
            let path = cache_path("hit");
            fs::write(&path, BODY).unwrap();
            let expected = digest_of(BODY);

            let bytes = load(&Client::new(), UNREACHABLE, &path, Some(&expected))
                .await
                .expect("摘要匹配的缓存应直接命中");
            assert_eq!(bytes, BODY);
            assert!(path.is_file(), "命中的缓存不应被删除");
            let _ = fs::remove_file(&path);
        });
    }

    #[test]
    fn legacy_cache_without_digest_keeps_previous_behavior() {
        tauri::async_runtime::block_on(async {
            let path = cache_path("legacy");
            fs::write(&path, OTHER).unwrap();

            // 老索引没有摘要：只要能解析就沿用，不发起网络（若回源，UNREACHABLE 必失败）
            let bytes = load(&Client::new(), UNREACHABLE, &path, None)
                .await
                .expect("legacy 路径应沿用可解析的缓存");
            assert_eq!(bytes, OTHER);
            let _ = fs::remove_file(&path);
        });
    }

    #[test]
    fn stale_cache_with_wrong_digest_is_deleted_and_refetched() {
        tauri::async_runtime::block_on(async {
            let path = cache_path("stale");
            fs::write(&path, OTHER).unwrap();
            let server = MockServer::spawn(BODY.to_vec());
            let expected = digest_of(BODY);

            let bytes = load(&Client::new(), &server.url, &path, Some(&expected))
                .await
                .expect("摘要不符应回源");
            assert_eq!(bytes, BODY, "应拿到线上那份，而不是缓存里的旧那份");
            assert_eq!(fs::read(&path).unwrap(), BODY, "回源通过后应覆盖缓存");

            server.handle.join().unwrap();
            let _ = fs::remove_file(&path);
        });
    }

    #[test]
    fn unparsable_cache_is_deleted_and_refetched() {
        tauri::async_runtime::block_on(async {
            let path = cache_path("garbage-cache");
            fs::write(&path, b"<html>502 Bad Gateway</html>").unwrap();
            let server = MockServer::spawn(BODY.to_vec());

            // 无摘要时"能解析"是唯一自愈信号，不能被丢掉
            let bytes = load(&Client::new(), &server.url, &path, None).await.expect("坏缓存应回源");
            assert_eq!(bytes, BODY);
            assert_eq!(fs::read(&path).unwrap(), BODY);

            server.handle.join().unwrap();
            let _ = fs::remove_file(&path);
        });
    }

    #[test]
    fn digest_mismatch_after_refetch_is_used_but_not_cached() {
        tauri::async_runtime::block_on(async {
            let path = cache_path("mismatch");
            let server = MockServer::spawn(BODY.to_vec());
            let wrong = digest_of(OTHER); // 索引抄错 / 清单被覆盖上传后没重算索引

            let bytes = load(&Client::new(), &server.url, &path, Some(&wrong))
                .await
                .expect("摘要不符不应阻断更新（宽松档语义）");
            assert_eq!(bytes, BODY, "仍然使用线上那份字节");
            assert!(!path.exists(), "摘要不符的字节不得固化进缓存");

            server.handle.join().unwrap();
        });
    }

    #[test]
    fn remote_non_json_is_error() {
        tauri::async_runtime::block_on(async {
            let path = cache_path("bad-remote");
            let server = MockServer::spawn(b"<html>404 Not Found</html>".to_vec());

            let error = load(&Client::new(), &server.url, &path, None)
                .await
                .expect_err("非 JSON 应报错");
            assert!(
                format!("{error:#}").contains("清单格式不符合规范"),
                "错误应指出清单非法: {error}"
            );
            assert!(!path.exists(), "非法字节不写缓存");

            server.handle.join().unwrap();
        });
    }

    #[test]
    fn atomic_write_replaces_existing_and_leaves_no_tmp() {
        let dir = app_paths::temp_dir().join("update/atomic");
        let path = dir.join("0.8.6.json");
        write_atomic(&path, OTHER).expect("首次写入应成功");
        write_atomic(&path, BODY).expect("覆盖写入应成功");

        assert_eq!(fs::read(&path).unwrap(), BODY, "应原子覆盖为最新字节");
        assert!(!dir.join("0.8.6.json.tmp").exists(), "临时文件不应残留");
        let _ = fs::remove_dir_all(&dir);
    }
}
