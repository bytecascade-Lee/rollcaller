//! 下载更新产物：流式下载到临时文件（.part）→ 校验（sha256 + 签名）→ 重命名为正式文件名
//!
//! # 落盘策略
//!
//! 下载物先写入 `target_dir` 下的 `{随机id}.part`——无信息量的名字、非可执行扩展名，顺序与内容不可被外部按文件名推断；
//! 完整写盘并校验**通过后**才 `rename` 为 url 最后一段的正式文件名。
//! 正式落点一定是已验证字节；校验失败 / 取消即删除 `.part`，磁盘上从不出现"未验证的正式产物"。
//!
//! # 传输与超时
//!
//! 使用 [`http_client::download`]：**不设整体超时**，只设连接超时与空闲读超时。
//! 无断点续传（GitHub/CNB release 资产不保证支持 Range），失败 / 取消即全量重下。
//!
//! # 校验
//!
//! sha256 流式读文件比对；`artifact.signature` 非空时再做 minisign 签名验证（公钥来自 `verify::pubkey()`）。
//! 主包产物带签名；Go updater（更新器）目前只有 sha256，走同一函数（signature 为空即跳过签名）。

use crate::common::entity::update::Artifact;
use crate::state::http_client;
use crate::updater::verify::{pubkey, verify_signature};
use anyhow::{anyhow, Context};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 下载进度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    /// 已下载字节数
    pub downloaded: u64,
    /// 总字节数（来自 Content-Length，未知时为 None）
    pub total: Option<u64>,
}

/// 随机临时文件名（无信息量；写入顺序不泄露给按文件名观察者）
fn random_part_name() -> String {
    format!("{}.part", hex::encode(rand::random::<u128>().to_le_bytes()))
}

/// 流式下载 `artifact.url` 到 `target_dir`，校验后重命名为正式文件名并返回其路径
///
/// # 流程
/// 1. 目标文件名取 url 最后一段（与发布清单中的产物名一致）；
/// 2. 下载全程写入 `target_dir/{随机id}.part`（每 chunk 写盘、上报进度、检查 `cancel`）；
/// 3. 下载完成对**文件**流式校验：sha256 必验；`signature` 非空时再验 minisign 签名；
/// 4. 通过后 `rename` 为正式名（已存在同名先删除），失败/取消即删除 `.part`。
///
/// # 取消
/// `cancel` 由调用方持有（前端 `cancel_update` 置位）。下载中途发现置位 → 清理 `.part`
/// 并返回错误（文案 `CANCELLED`，前端可据此静默回到可重下状态）。下载完成后置位
/// 无效——产物已落盘，由调用方的取消流程另行删除。
pub async fn download(
    artifact: &Artifact,
    target_dir: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(DownloadProgress),
) -> anyhow::Result<PathBuf> {
    // 1. 正式文件名 = url 最后一段（含扩展名，与清单产物同名）
    let file_name = artifact
        .url
        .path_segments()
        .and_then(|segments| segments.last())
        .filter(|name| !name.is_empty())
        .context(anyhow!("下载地址缺少文件名: {}", artifact.url))?;
    let final_path = target_dir.join(file_name);

    // 2. 下载到随机 .part（同名残留天然隔离，无需清理历史）
    std::fs::create_dir_all(target_dir).map_err(|e| anyhow!("创建下载目录失败（{}）：{e}", target_dir.display()))?;
    let part_path = target_dir.join(random_part_name());

    let response = http_client::download()
        .get(artifact.url.as_str())
        .send()
        .await
        .map_err(|e| anyhow!("下载失败（网络错误）：{e}"))?;
    if !response.status().is_success() {
        return Err(anyhow!("下载失败：服务器返回 HTTP {}", response.status()));
    }

    let total = response.content_length();
    let mut file = File::create(&part_path).map_err(|e| anyhow!("创建下载临时文件失败（{}）：{e}", part_path.display()))?;
    let mut downloaded: u64 = 0;
    let mut stream = response;
    while let Some(chunk) = stream.chunk().await.map_err(|e| anyhow!("下载中断：{e}"))? {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&part_path);
            return Err(anyhow!("CANCELLED"));
        }
        file.write_all(&chunk).map_err(|e| anyhow!("写入下载临时文件失败：{e}"))?;
        downloaded += chunk.len() as u64;
        on_progress(DownloadProgress { downloaded, total });
    }
    drop(file);

    // 3. 校验（文件流式）；失败删除 .part，不留下任何未验证产物
    verify_file(&part_path, artifact).map_err(|e| {
        let _ = std::fs::remove_file(&part_path);
        e.context(anyhow!("下载内容校验失败"))
    })?;

    // 4. rename 为正式名（Windows rename 不覆盖已存在文件，先清同名残留）
    if final_path.exists() {
        let _ = std::fs::remove_file(&final_path);
    }
    std::fs::rename(&part_path, &final_path).map_err(|e| {
        anyhow!(
            "重命名下载产物失败（{} → {}）：{e}",
            part_path.display(),
            final_path.display()
        )
    })?;
    Ok(final_path)
}

/// 对已落盘的临时文件做校验：sha256 流式（必验）+ minisign 签名（signature 非空时）
fn verify_file(path: &Path, artifact: &Artifact) -> anyhow::Result<()> {
    let mut file = File::open(path).map_err(|e| anyhow!("读取下载产物失败（{}）：{e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| anyhow!("计算下载产物哈希失败：{e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = hex::encode(hasher.finalize());
    if actual != artifact.sha256.to_ascii_lowercase() {
        anyhow::bail!("sha256 不匹配: 期望 {}, 实际 {}", artifact.sha256, actual);
    }

    // 签名验证：minisign-verify 该版本仅支持整段字节（&[u8]），此处一次性读入；
    // 主包产物带签名，Go updater（更新器）signature 为空则跳过。
    if !artifact.signature.is_empty() {
        let data = std::fs::read(path).map_err(|e| anyhow!("读取下载产物失败（{}）：{e}", path.display()))?;
        verify_signature(&data, &artifact.signature, pubkey())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::thread::JoinHandle;
    use std::time::Duration;
    use tiny_http::{Response, Server, StatusCode};
    use url::Url;

    /// 轻量 mock 下载服务器：返回一段字节流后自动关闭
    struct MockServer {
        url: String,
        handle: JoinHandle<()>,
    }

    impl MockServer {
        fn spawn_bytes(body: Vec<u8>, status: u16) -> Self {
            let server = Server::http("127.0.0.1:0").expect("mock 服务器启动失败");
            let port = server.server_addr().to_ip().expect("无法获取端口").port();
            let url = format!("http://127.0.0.1:{port}/update.bin");
            let handle = std::thread::spawn(move || {
                if let Ok(Some(request)) = server.recv_timeout(Duration::from_secs(10)) {
                    let response = Response::from_data(body).with_status_code(StatusCode(status));
                    let _ = request.respond(response);
                }
            });
            Self { url, handle }
        }
    }

    fn artifact_for(url: &str, body: &[u8], sha256: Option<&str>) -> Artifact {
        Artifact {
            url: Url::parse(url).unwrap(),
            sha256: sha256.unwrap_or(&hex::encode(Sha256::digest(body))).to_string(),
            signature: String::new(), // 测试仅覆盖 sha256 路径（签名路径同 verify.rs 已测）
            size: body.len() as u64,
        }
    }

    fn temp_target(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rollcaller-dl-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("创建测试目录失败");
        dir
    }

    fn leftover_parts(dir: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .map(|it| {
                it.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "part"))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn download_writes_final_file_and_reports_progress() {
        tauri::async_runtime::block_on(async {
            let body = b"hello world, rollcaller update!".to_vec();
            let server = MockServer::spawn_bytes(body.clone(), 200);
            let dir = temp_target("ok");
            let cancel = AtomicBool::new(false);
            let mut last: Option<DownloadProgress> = None;

            let path = download(&artifact_for(&server.url, &body, None), &dir, &cancel, |p| last = Some(p))
                .await
                .expect("下载不应失败");

            // 正式文件名 = url 最后一段，无 .part 残留
            assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("update.bin"));
            assert_eq!(std::fs::read(&path).unwrap(), body, "落盘内容应与 mock 一致");
            assert!(leftover_parts(&dir).is_empty(), "不应残留 .part");

            let last = last.expect("应有进度回调");
            assert_eq!(last.downloaded, body.len() as u64, "累计进度应等于字节数");
            assert_eq!(last.total, Some(body.len() as u64), "Content-Length 应被解析");

            let _ = std::fs::remove_dir_all(&dir);
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_errors_on_http_error_and_leaves_nothing() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_bytes(Vec::new(), 404);
            let dir = temp_target("http404");
            let cancel = AtomicBool::new(false);
            let body = b"whatever".to_vec();

            let err = download(&artifact_for(&server.url, &body, None), &dir, &cancel, |_| {})
                .await
                .expect_err("404 应报错");
            assert!(err.to_string().contains("404"), "错误信息应可读: {err}");
            assert!(
                std::fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(true),
                "目录应为空"
            );

            let _ = std::fs::remove_dir_all(&dir);
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_cancel_removes_part() {
        tauri::async_runtime::block_on(async {
            let body = b"payload that will be cancelled".to_vec();
            let server = MockServer::spawn_bytes(body.clone(), 200);
            let dir = temp_target("cancel");
            // 预先置位：首个 chunk 到达即中断
            let cancel = AtomicBool::new(true);

            let err = download(&artifact_for(&server.url, &body, None), &dir, &cancel, |_| {})
                .await
                .expect_err("置位取消应报错");
            assert!(err.to_string().contains("CANCELLED"), "取消文案应可识别: {err}");
            assert!(leftover_parts(&dir).is_empty(), "取消后不应残留 .part");
            assert!(
                std::fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(true),
                "目录应为空"
            );

            let _ = std::fs::remove_dir_all(&dir);
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_rejects_wrong_sha256_and_leaves_nothing() {
        tauri::async_runtime::block_on(async {
            let body = b"tampered payload".to_vec();
            let server = MockServer::spawn_bytes(body.clone(), 200);
            let dir = temp_target("sha256");
            let cancel = AtomicBool::new(false);
            let wrong = "00".repeat(32);

            let err = download(&artifact_for(&server.url, &body, Some(&wrong)), &dir, &cancel, |_| {})
                .await
                .expect_err("sha256 不匹配应报错");
            assert!(
                err.chain().any(|c| c.to_string().contains("sha256 不匹配")),
                "错误链应含 sha256 不匹配: {err:?}"
            );
            assert!(
                std::fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(true),
                "目录应为空"
            );

            let _ = std::fs::remove_dir_all(&dir);
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn random_part_names_are_unique() {
        let a = random_part_name();
        let b = random_part_name();
        assert_ne!(a, b);
        assert!(a.ends_with(".part"));
    }
}
