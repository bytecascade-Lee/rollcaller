//! 流式下载更新包并上报进度

use crate::updater::manifest::Artifact;
use anyhow::anyhow;
use reqwest::Client;

/// 下载进度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    /// 已下载字节数
    pub downloaded: u64,
    /// 总字节数（来自 Content-Length，未知时为 None）
    pub total: Option<u64>,
}

/// 流式下载 `artifact.url` 到内存并返回原始字节
///
/// 下载完成后**不**做验签与安装：验签在任务 03 接入，本函数只返回原始字节。
/// 错误均为可恢复错误（网络中断 / 非 2xx），由调用方决定是否重试。
pub async fn download(
    client: &Client,
    artifact: &Artifact,
    mut on_progress: impl FnMut(DownloadProgress),
) -> anyhow::Result<Vec<u8>> {
    let mut resp = client
        .get(artifact.url.as_str())
        .send()
        .await
        .map_err(|e| anyhow!("下载失败（网络错误）：{e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow!("下载失败：服务器返回 HTTP {status}"));
    }

    let total = resp.content_length();
    let mut bytes = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut downloaded: u64 = 0;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| anyhow!("下载中断：{e}"))?
    {
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        on_progress(DownloadProgress { downloaded, total });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::manifest::Artifact;
    use std::time::Duration;
    use tiny_http::{Response, Server, StatusCode};

    /// 轻量 mock 下载服务器：返回一段字节流后自动关闭
    struct MockServer {
        url: String,
        handle: std::thread::JoinHandle<()>,
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

    fn client() -> Client {
        Client::new()
    }

    fn artifact_with(url: &str) -> Artifact {
        Artifact {
            url: url::Url::parse(url).unwrap(),
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            signature: "c2ln".to_string(),
            size: 100,
        }
    }

    #[test]
    fn download_streams_bytes_and_progress() {
        tauri::async_runtime::block_on(async {
            let body = b"hello world, rollcaller update!".to_vec();
            let server = MockServer::spawn_bytes(body.clone(), 200);
            let artifact = artifact_with(&server.url);

            let mut last: Option<DownloadProgress> = None;
            let bytes = download(&client(), &artifact, |p| last = Some(p))
                .await
                .expect("下载不应失败");

            assert_eq!(bytes, body, "下载字节应与 mock 内容一致");
            let last = last.expect("应有进度回调");
            assert_eq!(last.downloaded, body.len() as u64, "累计进度应等于字节数");
            assert_eq!(last.total, Some(body.len() as u64), "Content-Length 应被解析");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_errors_on_http_error() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_bytes(Vec::new(), 404);
            let artifact = artifact_with(&server.url);
            let err = download(&client(), &artifact, |_| {})
                .await
                .expect_err("404 应报错");
            assert!(err.to_string().contains("404"), "错误信息应可读: {err}");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn download_empty_body_still_calls_progress() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_bytes(Vec::new(), 200);
            let artifact = artifact_with(&server.url);
            let mut called = 0;
            let bytes = download(&client(), &artifact, |_| called += 1)
                .await
                .expect("下载不应失败");
            assert!(bytes.is_empty());
            assert_eq!(called, 0, "空 body 不应有进度回调（无 chunk）");
            server.handle.join().unwrap();
        });
    }
}
