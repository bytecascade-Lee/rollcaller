//! 下载更新产物：流式写入 `.part` → 校验 → 重命名为正式产物。
//!
//! 下载中的文件使用 `{随机uuid}.part` 命名、非可执行扩展名，且与正式产物分居两个目录，
//! 只有通过校验的字节才会以正式文件名出现，失败或取消即全量重下。
//!
//! 暂时不做断点续传（CNB 无 `Range` 支持）

use crate::state::http_client;
use crate::updater::model::{Artifact, DownloadProgress};
use crate::updater::paths;
use crate::updater::verify::verify_artifact_path;
use anyhow::anyhow;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// 用户取消下载
///
/// 用类型而非错误文案传递：编排层据 `downcast_ref` 回到可重下状态，改文案不会破协议。
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("下载已取消")
    }
}

impl std::error::Error for Cancelled {}

/// 流式下载 `artifact.url`，校验后落为正式产物并返回其路径
///
/// 正式产物已存在且验签通过时幂等返回；卸载后置位的取消不再生效，彼时产物已落盘，由调用方另行删除。
pub async fn download(
    artifact: &Artifact,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(DownloadProgress),
) -> anyhow::Result<PathBuf> {
    let part_path = paths::part();
    let final_path = paths::package_of(artifact)?;

    // part 与 final 分居两个目录，父目录都可能尚未创建
    if let Some(parent) = part_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow!("创建下载工作目录失败（{}）：{e}", parent.display()))?;
    }
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow!("创建产物目录失败（{}）：{e}", parent.display()))?;
    }

    // 同名残留，验签通过即复用，否则删掉重下
    if final_path.exists() {
        match verify_artifact_path(&final_path, artifact) {
            Ok(_) => return Ok(final_path),
            Err(_) => std::fs::remove_file(&final_path)
                .map_err(|e| anyhow!("删除残留的错误文件 {} 失败：{e}", final_path.display()))?,
        }
    }

    let response = http_client::download()
        .get(artifact.url.as_str())
        .send()
        .await
        .map_err(|e| anyhow!("下载失败：网络错误 {e}"))?;
    if !response.status().is_success() {
        return Err(anyhow!("下载失败：服务器返回 HTTP {}", response.status()));
    }

    let total = response.content_length();
    let mut file = File::create(&part_path).map_err(|e| anyhow!("创建临时下载文件 {} 失败：{e}", part_path.display()))?;
    let mut downloaded: u64 = 0;
    let mut stream = response;
    while let Some(chunk) = stream.chunk().await.map_err(|e| anyhow!("下载中断：{e}"))? {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&part_path);
            return Err(anyhow::Error::new(Cancelled));
        }
        file.write_all(&chunk)
            .map_err(|e| anyhow!("写入临时下载文件 {} 失败：{e}", part_path.display()))?;
        downloaded += chunk.len() as u64;
        on_progress(DownloadProgress { downloaded, total });
    }
    drop(file);

    // 校验失败即删 .part，磁盘上不留未验证的字节
    verify_artifact_path(&part_path, artifact).map_err(|e| {
        let _ = std::fs::remove_file(&part_path);
        e.context(anyhow!("下载内容校验失败，已删除。"))
    })?;

    // 同卷跨目录 rename 原子；同名残留在前面已处理
    std::fs::rename(&part_path, &final_path).map_err(|e| {
        anyhow!(
            "重命名下载产物失败（{} → {}）：{e}",
            part_path.display(),
            final_path.display()
        )
    })?;
    Ok(final_path)
}

/// 就绪锚点判定：产物文件存在 -> 先以 size 短路 -> 再整体验签
///
/// 返回 `Some(路径)` 表示"已下载待安装"；
/// 不存在 / 校验失败均返回 `None`，且校验失败或 size 不符时清除文件待重下。
pub fn ready(path: &Path, artifact: &Artifact) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    let size_ok = path.metadata().map(|m| m.len()).ok() == Some(artifact.size);
    if !size_ok {
        let _ = std::fs::remove_file(path);
        return None;
    }
    match verify_artifact_path(path, artifact) {
        Ok(()) => Some(path.to_path_buf()),
        Err(_) => {
            let _ = std::fs::remove_file(path);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::verify::test_keys;
    use sha2::{Digest, Sha256};
    use std::sync::atomic::AtomicBool;
    use std::thread::JoinHandle;
    use std::time::Duration;
    use tiny_http::{Response, Server, StatusCode};
    use url::Url;

    /// 轻量 mock 下载服务器：url 文件名 = tag（产物名由此唯一，共享测试根下用例间互不撞车）
    struct MockServer {
        url: String,
        handle: JoinHandle<()>,
    }

    impl MockServer {
        fn spawn_bytes(tag: &str, body: Vec<u8>, status: u16) -> Self {
            let server = Server::http("127.0.0.1:0").expect("mock 服务器启动失败");
            let port = server.server_addr().to_ip().expect("无法获取端口").port();
            let url = format!("http://127.0.0.1:{port}/{tag}.bin");
            let handle = std::thread::spawn(move || {
                if let Ok(Some(request)) = server.recv_timeout(Duration::from_secs(10)) {
                    let response = Response::from_data(body).with_status_code(StatusCode(status));
                    let _ = request.respond(response);
                }
            });
            Self { url, handle }
        }
    }

    /// 构造 artifact（url 指向 mock 服务器或不可达端口；url 文件名 = 产物名）
    ///
    /// 签名用**测试构建的有效密钥对**（[`test_keys`]）真实签出：主包产物签名必填，
    /// 空签名会在校验入口被直接拒绝（回归用例见 `download_rejects_unsigned_artifact`）。
    fn artifact_for(url: &str, body: &[u8], sha256: Option<&str>) -> Artifact {
        Artifact {
            url: Url::parse(url).unwrap(),
            sha256: sha256.unwrap_or(&hex::encode(Sha256::digest(body))).to_string(),
            signature: test_keys::sign(body),
            size: body.len() as u64,
        }
    }

    /// Test Mode 数据根下该产物应落的正式路径（= paths::package(url 文件名)）
    fn expect_package_path(tag: &str) -> PathBuf {
        paths::package(&format!("{tag}.bin"))
    }

    #[test]
    fn download_writes_final_file_and_reports_progress() {
        tauri::async_runtime::block_on(async {
            let body = b"hello world, rollcaller update!".to_vec();
            let server = MockServer::spawn_bytes("ok", body.clone(), 200);
            let cancel = AtomicBool::new(false);
            let mut last: Option<DownloadProgress> = None;

            let path = download(&artifact_for(&server.url, &body, None), &cancel, |p| last = Some(p))
                .await
                .expect("下载不应失败");

            // 正式产物落于 packages/{mock 文件名}；内容与 mock 一致
            let final_path = expect_package_path("ok");
            assert_eq!(path, final_path, "返回路径应为 packages 落点");
            assert_eq!(std::fs::read(&final_path).unwrap(), body, "落盘内容应与 mock 一致");

            let last = last.expect("应有进度回调");
            assert_eq!(last.downloaded, body.len() as u64, "累计进度应等于字节数");
            assert_eq!(last.total, Some(body.len() as u64), "Content-Length 应被解析");

            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_skips_when_final_file_already_valid() {
        tauri::async_runtime::block_on(async {
            // final 已存在且校验通过 → 不应发起任何网络请求（url 指向不可达端口，
            // 若真被请求会立刻报网络错误）
            let body = b"already downloaded".to_vec();
            let cancel = AtomicBool::new(false);
            let final_path = expect_package_path("skip");
            std::fs::create_dir_all(final_path.parent().unwrap()).unwrap();
            std::fs::write(&final_path, &body).unwrap();

            let path = download(&artifact_for("http://127.0.0.1:1/skip.bin", &body, None), &cancel, |_| {})
                .await
                .expect("final 已就绪应直接返回");
            assert_eq!(path, final_path);
        });
    }

    #[test]
    fn download_errors_on_http_error_and_leaves_nothing() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_bytes("http404", Vec::new(), 404);
            let cancel = AtomicBool::new(false);
            let body = b"whatever".to_vec();

            let err = download(&artifact_for(&server.url, &body, None), &cancel, |_| {})
                .await
                .expect_err("404 应报错");
            assert!(err.to_string().contains("404"), "错误信息应可读: {err}");
            assert!(!expect_package_path("http404").exists(), "HTTP 失败不应留下正式产物");

            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_cancel_removes_part() {
        tauri::async_runtime::block_on(async {
            let body = b"payload that will be cancelled".to_vec();
            let server = MockServer::spawn_bytes("cancel", body.clone(), 200);
            // 预先置位：首个 chunk 到达即中断
            let cancel = AtomicBool::new(true);

            let err = download(&artifact_for(&server.url, &body, None), &cancel, |_| {})
                .await
                .expect_err("置位取消应报错");
            assert!(
                err.downcast_ref::<Cancelled>().is_some(),
                "应以 Cancelled 类型报告取消: {err}"
            );
            assert!(!expect_package_path("cancel").exists(), "取消后不应留下正式产物");

            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_rejects_wrong_sha256_and_leaves_nothing() {
        tauri::async_runtime::block_on(async {
            let body = b"tampered payload".to_vec();
            let server = MockServer::spawn_bytes("sha256", body.clone(), 200);
            let cancel = AtomicBool::new(false);
            let wrong = "00".repeat(32);

            let err = download(&artifact_for(&server.url, &body, Some(&wrong)), &cancel, |_| {})
                .await
                .expect_err("sha256 不匹配应报错");
            assert!(
                err.chain().any(|c| c.to_string().contains("sha256 不匹配")),
                "错误链应含 sha256 不匹配: {err:?}"
            );
            assert!(!expect_package_path("sha256").exists(), "校验失败不应留下正式产物");

            server.handle.join().unwrap();
        });
    }

    /// 回归：**空签名产物必须被拒绝**（fail closed，不得"跳过验签只比 sha256"）
    ///
    /// 背景：清单落盘缓存且优先于网络；若允许空签名跳过 minisign，改写缓存的人只需把
    /// `sha256` 换成自己产物的哈希、`signature` 清空，即可让任意 exe 通过校验并被安装。
    #[test]
    fn download_rejects_unsigned_artifact() {
        tauri::async_runtime::block_on(async {
            let body = b"unsigned payload".to_vec();
            let server = MockServer::spawn_bytes("unsigned", body.clone(), 200);
            let cancel = AtomicBool::new(false);
            let artifact = Artifact {
                signature: String::new(),
                ..artifact_for(&server.url, &body, None)
            };

            let err = download(&artifact, &cancel, |_| {}).await.expect_err("空签名应被拒绝");
            assert!(
                err.chain().any(|c| c.to_string().contains("缺少 minisign 签名")),
                "错误链应指出签名缺失: {err:?}"
            );
            assert!(!expect_package_path("unsigned").exists(), "签名缺失不得留下正式产物");

            server.handle.join().unwrap();
        });
    }
}
