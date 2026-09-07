//! 下载更新产物：流式下载到临时文件（.part）→ 校验 → 重命名为正式文件名
//!
//! # 落盘策略
//!
//! 下载物先完整写入 `part_path` ——无信息量的名字、非可执行扩展名，
//! 顺序与内容不可被外部按文件名推断；完整写盘并校验**通过后**才 `rename` 为 `final_path`。
//! 正式落点一定是已验证字节；校验失败 / 取消即删除 `.part`，磁盘上从不出现"未验证的正式产物"。
//!
//! # 传输与超时
//!
//! 使用 [`http_client::download`]：**不设整体超时**，只设连接超时与空闲读超时。
//!
//! 暂时**不实现断点续传**，下载失败或取消即全量重下。具体平台能力如下：
//! - **GitHub**：**支持断点续传**。
//!   请求原始下载链接（如 `https://github.com/owner/repo/releases/download/...` ）会返回 **302 重定向**，
//!   最终指向 `release-assets.githubusercontent.com`（Azure CDN）。该 CDN 响应头包含 `Accept-Ranges: bytes`，
//!   明确支持 `Range` 请求。若在重试时重新请求原始链接（自动跟随重定向获取新令牌）并携带 `Range` 头，
//!   即可实现从断点处续传。
//! - **CNB**：**不支持断点续传**。
//!   经实测，即使客户端携带 `Range: bytes=0-99` 请求头，服务端仍返回 `HTTP 200 OK` 及完整的文件内容
//!   （`Content-Length` 为全量大小），而非 `206 Partial Content`。因此在该平台上下载中断后无法续传。
//!
//! 未来如果需要扩展，可在 `Github` 上实现多线程下载及断点续传策略。
//!
//! # 校验
//!
//! 校验逻辑在 [`verify_artifact_path`]：整段读入已落盘的 .part 后
//! sha256 必验；`signature` 非空时再做 minisign 签名验证（双重 base64 处理见该模块）。
//! 主包产物带签名；Go updater（更新器）目前只有 sha256（signature 为空即跳过签名）。

use crate::common::entity::update::Artifact;
use crate::service::update::paths;
use crate::service::update::verify::verify_artifact_path;
use crate::state::http_client;
use anyhow::anyhow;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// 下载进度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    /// 已下载字节数
    pub downloaded: u64,
    /// 总字节数（来自 Content-Length，未知时为 None）
    pub total: Option<u64>,
}

/// 流式下载 `artifact.url` 到 `part_path`，校验后重命名为 `final_path` 并返回其路径
///
/// # 参数
/// - `artifact`：下载凭据（url 为下载源；sha256 / signature 供落盘后整体校验）。
/// - `part_path`：下载工作区路径（调用方经 `paths::part()` 生成随机名，下载中数据暂存于此）。
/// - `final_path`：正式产物路径（调用方经 `paths::artifact_package_path()` 给出，校验通过后 rename 落定）。
/// - `cancel`：取消信号（前端 `cancel` 命令置位），下载中途发现置位 → 清理 `.part`
///   并返回错误（文案 `CANCELLED`，前端可据此静默回到可重下状态）。
/// - `on_progress`：进度回调（每 chunk 上报）。
///
/// # 流程
/// 1. `final_path` 已存在 → 先整体验签：通过即视为已下载（幂等返回），失败则删除待重下；
/// 2. 下载全程写入 `part_path`（每 chunk 写盘、上报进度、检查 `cancel`）；
/// 3. 下载完成对 `part_path` 整读校验（sha256 必验；`signature` 非空再验 minisign 签名），
///    失败删除 `.part`；
/// 4. 校验通过 `rename` 为 `final_path`（已存在同名残留已在步骤 1 处理）。
///
/// # 取消
/// 下载完成后置位无效——产物已落盘，由调用方的取消流程另行删除。
pub async fn download(
    artifact: &Artifact,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(DownloadProgress),
) -> anyhow::Result<PathBuf> {
    let part_path = paths::part();

    let final_path = paths::package(match artifact.file_name() {
        Some(path) => &path,
        None => return Err(anyhow!("下载地址缺少文件名: {}", artifact.url))
    });
    // 0. 确保 part 与 final 的父目录存在（分居两目录，均可能尚未创建）
    if let Some(parent) = part_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("创建下载工作目录失败（{}）：{e}", parent.display()))?;
    }
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("创建产物目录失败（{}）：{e}", parent.display()))?;
    }

    // 1. final 残留已就绪：校验通过直接返回（不重复下载）；失败删除重下
    if final_path.exists() {
        match verify_artifact_path(&final_path, artifact) {
            Ok(_) => return Ok(final_path),
            Err(_) => std::fs::remove_file(&final_path)
                .map_err(|e| anyhow!("删除残留的错误文件 {} 失败：{e}", final_path.display()))?,
        }
    }

    // 2. 请求下载源
    let response = http_client::download()
        .get(&artifact.url)
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
            return Err(anyhow!("CANCELLED"));
        }
        file.write_all(&chunk).map_err(|e| anyhow!("写入临时下载文件 {} 失败：{e}", part_path.display()))?;
        downloaded += chunk.len() as u64;
        on_progress(DownloadProgress { downloaded, total });
    }
    drop(file);

    // 4. 校验（verify 模块 path 入口：整读后 sha256 + 可选签名）；失败删除 .part
    verify_artifact_path(&part_path, artifact).map_err(|e| {
        let _ = std::fs::remove_file(&part_path);
        e.context(anyhow!("下载内容校验失败，已删除。"))
    })?;

    // 5. rename 为正式名（前面已处理同名残留，此处无需再判断）
    std::fs::rename(&part_path, &final_path).map_err(|e| {
        anyhow!(
            "重命名下载产物失败（{} → {}）：{e}",
            part_path.display(),
            final_path.display()
        )
    })?;
    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
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

    /// 独立的临时根：part 与 final 分置两个子目录，模拟真实布局的跨目录 rename
    fn temp_layout(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("rollcaller-dl-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let part_dir = root.join("parts");
        let final_dir = root.join("finals");
        (root, part_dir, final_dir)
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
            let (root, part_dir, final_dir) = temp_layout("ok");
            let cancel = AtomicBool::new(false);
            let part_path = part_dir.join("a1b2.part");
            let final_path = final_dir.join("update.bin");
            let mut last: Option<DownloadProgress> = None;

            let path = download(&artifact_for(&server.url, &body, None), &cancel, |p| last = Some(p))
                .await
                .expect("下载不应失败");

            // 正式产物落于 final 路径（文件名与 url 无关，由调用方给定），part 无残留
            assert_eq!(path, final_path);
            assert_eq!(std::fs::read(&final_path).unwrap(), body, "落盘内容应与 mock 一致");
            assert!(leftover_parts(&part_dir).is_empty(), "工作区不应残留 .part");

            let last = last.expect("应有进度回调");
            assert_eq!(last.downloaded, body.len() as u64, "累计进度应等于字节数");
            assert_eq!(last.total, Some(body.len() as u64), "Content-Length 应被解析");

            let _ = std::fs::remove_dir_all(&root);
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_skips_when_final_file_already_valid() {
        tauri::async_runtime::block_on(async {
            // final 已存在且校验通过 → 不应发起任何网络请求（url 指向不可达端口，
            // 若真被请求会立刻报网络错误）
            let body = b"already downloaded".to_vec();
            let (root, part_dir, final_dir) = temp_layout("skip");
            let cancel = AtomicBool::new(false);
            let final_path = final_dir.join("update.bin");
            std::fs::create_dir_all(&final_dir).unwrap();
            std::fs::write(&final_path, &body).unwrap();

            let artifact = artifact_for("http://127.0.0.1:1/update.bin", &body, None);
            let part_path = part_dir.join("c3d4.part");
            let path = download(&artifact, &cancel, |_| {})
                .await
                .expect("final 已就绪应直接返回");
            assert_eq!(path, final_path);
            assert!(leftover_parts(&part_dir).is_empty());

            let _ = std::fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn download_errors_on_http_error_and_leaves_nothing() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_bytes(Vec::new(), 404);
            let (root, part_dir, final_dir) = temp_layout("http404");
            let cancel = AtomicBool::new(false);
            let body = b"whatever".to_vec();
            let part_path = part_dir.join("e5f6.part");
            let final_path = final_dir.join("update.bin");

            let err = download(&artifact_for(&server.url, &body, None), &cancel, |_| {})
                .await
                .expect_err("404 应报错");
            assert!(err.to_string().contains("404"), "错误信息应可读: {err}");
            assert!(
                std::fs::read_dir(&final_dir).map(|mut d| d.next().is_none()).unwrap_or(true),
                "产物目录应为空"
            );
            assert!(
                std::fs::read_dir(&part_dir).map(|mut d| d.next().is_none()).unwrap_or(true),
                "工作区应为空"
            );

            let _ = std::fs::remove_dir_all(&root);
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_cancel_removes_part() {
        tauri::async_runtime::block_on(async {
            let body = b"payload that will be cancelled".to_vec();
            let server = MockServer::spawn_bytes(body.clone(), 200);
            let (root, part_dir, final_dir) = temp_layout("cancel");
            // 预先置位：首个 chunk 到达即中断
            let cancel = AtomicBool::new(true);
            let part_path = part_dir.join("g7h8.part");
            let final_path = final_dir.join("update.bin");

            let err = download(&artifact_for(&server.url, &body, None), &cancel, |_| {})
                .await
                .expect_err("置位取消应报错");
            assert!(err.to_string().contains("CANCELLED"), "取消文案应可识别: {err}");
            assert!(leftover_parts(&part_dir).is_empty(), "取消后工作区不应残留 .part");
            assert!(
                std::fs::read_dir(&final_dir).map(|mut d| d.next().is_none()).unwrap_or(true),
                "取消后产物目录应为空"
            );

            let _ = std::fs::remove_dir_all(&root);
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_rejects_wrong_sha256_and_leaves_nothing() {
        tauri::async_runtime::block_on(async {
            let body = b"tampered payload".to_vec();
            let server = MockServer::spawn_bytes(body.clone(), 200);
            let (root, part_dir, final_dir) = temp_layout("sha256");
            let cancel = AtomicBool::new(false);
            let wrong = "00".repeat(32);
            let part_path = part_dir.join("i9j0.part");
            let final_path = final_dir.join("update.bin");

            let err = download(&artifact_for(&server.url, &body, Some(&wrong)), &cancel, |_| {})
                .await
                .expect_err("sha256 不匹配应报错");
            assert!(
                err.chain().any(|c| c.to_string().contains("sha256 不匹配")),
                "错误链应含 sha256 不匹配: {err:?}"
            );
            assert!(leftover_parts(&part_dir).is_empty(), "校验失败后工作区不应残留 .part");
            assert!(
                std::fs::read_dir(&final_dir).map(|mut d| d.next().is_none()).unwrap_or(true),
                "校验失败后产物目录应为空"
            );

            let _ = std::fs::remove_dir_all(&root);
            server.handle.join().unwrap();
        });
    }
}
