//! 更新检查：HTTP 拉取自定义清单 → 版本判定 → 返回更新信息

use crate::config::app_paths::{current_mode, AppMode};
use crate::updater::manifest::{current_arch_key, ArchPayloads, Artifact, UpdateManifest};
use crate::updater::version_policy::{decide, UpdateDecision};
use anyhow::anyhow;
use reqwest::Client;
use semver::Version;
use serde::Serialize;

/// 安装形态，前端展示用（序列化为 `"nsis"` | `"portable"`，任务 04 命令契约）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateKind {
    /// NSIS 安装包
    Nsis,
    /// 便携版 zip
    Portable,
}

/// 更新信息（check 的返回值；经 Tauri 命令序列化透传给前端）
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub version: Version,
    /// releaseNotes
    pub notes: Option<String>,
    /// publishDate（格式化后的字符串，供前端展示）
    pub date: Option<String>,
    /// 本次要下载的产物（nsis 或 portable 之一）
    pub artifact: Artifact,
    /// 原始清单，便于前端透传/调试
    pub raw_json: serde_json::Value,
    /// 本次安装形态
    pub kind: UpdateKind,
}

/// 占位端点：新清单文件名由仓库作者定（TODO: 后续接入配置）
pub const CHECK_ENDPOINT: &str =
    "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/<新清单文件名>";

/// 检查是否有可用更新
///
/// `current_version` 由调用处从 `app.package_info().version` 解析后传入，
/// 本函数保持纯函数风格，便于单元测试。
pub async fn check(
    client: &Client,
    endpoint: &str,
    current_version: &Version,
    allow_downgrade: bool,
) -> anyhow::Result<Option<UpdateInfo>> {
    do_check(client, endpoint, current_version, allow_downgrade, current_mode()).await
}

/// check 内部实现：运行模式由外部传入，便于测试非 Develop 路径
async fn do_check(
    client: &Client,
    endpoint: &str,
    current_version: &Version,
    allow_downgrade: bool,
    mode: AppMode,
) -> anyhow::Result<Option<UpdateInfo>> {
    // 1. 拉取清单（区分网络失败与清单格式错误）
    let resp = client
        .get(endpoint)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| anyhow!("检查更新失败（网络错误）：{e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow!("检查更新失败：清单服务器返回 HTTP {status}"));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| anyhow!("检查更新失败（读取响应失败）：{e}"))?;

    // 2. 解析清单（复用任务 01 类型）
    let raw_json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| anyhow!("检查更新失败（清单不是合法 JSON）：{e}"))?;
    let manifest: UpdateManifest = serde_json::from_value(raw_json.clone())
        .map_err(|e| anyhow!("检查更新失败（清单格式不符合规范）：{e}"))?;

    // 3. 当前架构无产物 → 无可用更新（不报错）
    let Some(arch) = current_arch_key() else {
        return Ok(None);
    };
    let Some(payloads) = manifest.artifact_for(arch) else {
        return Ok(None);
    };

    // 4. 按运行模式选择产物；无产物 → 无可用更新
    let Some((kind, artifact)) = select_artifact(payloads, mode) else {
        return Ok(None);
    };

    // 5. 版本判定
    match decide(current_version, &manifest.version, allow_downgrade) {
        UpdateDecision::Skip => Ok(None),
        UpdateDecision::Update => Ok(Some(UpdateInfo {
            version: manifest.version.clone(),
            notes: manifest.release_notes.clone(),
            date: manifest.publish_date.map(|d| d.to_string()),
            artifact,
            raw_json,
            kind,
        })),
    }
}

/// 按运行模式选择本次更新产物（纯函数，便于测试）
///
/// | current_mode() | 优先选 | 次选 | 都没有 |
/// |---|---|---|---|
/// | `Install` | nsis | 无 | None |
/// | `Portable` | portable | nsis（可迁移为安装版，迁移流在任务 09） | None |
/// | `Develop` | 不更新（None） | - | - |
fn select_artifact(payloads: &ArchPayloads, mode: AppMode) -> Option<(UpdateKind, Artifact)> {
    match mode {
        AppMode::Develop => None,
        AppMode::Install => payloads.nsis.clone().map(|a| (UpdateKind::Nsis, a)),
        AppMode::Portable => payloads
            .portable
            .clone()
            .map(|a| (UpdateKind::Portable, a))
            .or_else(|| payloads.nsis.clone().map(|a| (UpdateKind::Nsis, a))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tiny_http::{Header, Response, Server, StatusCode};

    /// 轻量 mock HTTP 服务器：按指定次数响应请求后自动关闭
    struct MockServer {
        url: String,
        handle: std::thread::JoinHandle<()>,
    }

    impl MockServer {
        fn spawn_json(body: &'static str, status: u16) -> Self {
            Self::spawn_json_n(body, status, 1)
        }

        fn spawn_json_n(body: &'static str, status: u16, n: usize) -> Self {
            Self::spawn_bytes_n(body.as_bytes().to_vec(), status, Some("application/json"), n)
        }

        fn spawn_bytes_n(
            body: Vec<u8>,
            status: u16,
            content_type: Option<&'static str>,
            n: usize,
        ) -> Self {
            let server = Server::http("127.0.0.1:0").expect("mock 服务器启动失败");
            let port = server.server_addr().to_ip().expect("无法获取端口").port();
            let url = format!("http://127.0.0.1:{port}/latest.json");
            let handle = std::thread::spawn(move || {
                let mut served = 0;
                while served < n {
                    match server.recv_timeout(Duration::from_secs(10)) {
                        Ok(Some(request)) => {
                            let mut response =
                                Response::from_data(body.clone()).with_status_code(StatusCode(status));
                            if let Some(ct) = content_type {
                                response = response.with_header(
                                    Header::from_bytes("Content-Type", ct).expect("header 合法"),
                                );
                            }
                            let _ = request.respond(response);
                            served += 1;
                        }
                        _ => break,
                    }
                }
            });
            Self { url, handle }
        }
    }

    fn client() -> Client {
        Client::new()
    }

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    /// 合法清单：x86_64 同时提供 nsis 与 portable，arm64 提供 nsis
    const MANIFEST_JSON: &str = r#"{
        "version": "1.2.0",
        "releaseNotes": "修复若干问题",
        "publishDate": "2026-08-01T00:00:00+08:00",
        "platforms": {
            "windows": {
                "x86_64": {
                    "nsis": {
                        "url": "https://example.com/app-x64-setup.exe",
                        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                        "signature": "c2ln",
                        "size": 100
                    },
                    "portable": {
                        "url": "https://example.com/app-x64-portable.zip",
                        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                        "signature": "c2ln",
                        "size": 100
                    }
                },
                "arm64": {
                    "nsis": {
                        "url": "https://example.com/app-arm64-setup.exe",
                        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                        "signature": "c2ln",
                        "size": 100
                    }
                }
            }
        }
    }"#;

    /// 只有 arm64 的清单（当前 x86_64 架构无产物）
    const MANIFEST_ARM64_ONLY: &str = r#"{
        "version": "1.2.0",
        "platforms": {
            "windows": {
                "arm64": {
                    "nsis": {
                        "url": "https://example.com/app-arm64-setup.exe",
                        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                        "signature": "c2ln",
                        "size": 100
                    }
                }
            }
        }
    }"#;

    fn artifact(name: &str) -> Artifact {
        Artifact {
            url: url::Url::parse(&format!("https://example.com/{name}")).unwrap(),
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            signature: "c2ln".to_string(),
            size: 100,
        }
    }

    #[test]
    fn check_returns_update_on_2xx() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_json(MANIFEST_JSON, 200);
            let info = do_check(&client(), &server.url, &v("1.0.0"), false, AppMode::Install)
                .await
                .expect("检查不应失败")
                .expect("应有可用更新");
            assert_eq!(info.version, v("1.2.0"));
            assert_eq!(info.notes.as_deref(), Some("修复若干问题"));
            assert!(info.date.is_some(), "publishDate 应格式化输出");
            assert_eq!(info.kind, UpdateKind::Nsis, "Install 模式应选 nsis");
            assert_eq!(info.artifact.url.as_str(), "https://example.com/app-x64-setup.exe");
            assert_eq!(info.raw_json["version"], "1.2.0", "应透传原始清单");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_errors_on_http_error() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_json("", 404);
            let err = do_check(&client(), &server.url, &v("1.0.0"), false, AppMode::Install)
                .await
                .expect_err("404 应报错");
            assert!(err.to_string().contains("404"), "错误信息应可读: {err}");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_errors_on_empty_body_2xx() {
        tauri::async_runtime::block_on(async {
            // 204 无 body → 2xx 但 JSON 解析失败（清单格式错误）
            let server = MockServer::spawn_json("", 204);
            let err = do_check(&client(), &server.url, &v("1.0.0"), false, AppMode::Install)
                .await
                .expect_err("空 body 应报错");
            assert!(err.to_string().contains("JSON"), "错误信息应可读: {err}");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_errors_on_bad_manifest() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_json("not a json", 200);
            let err = do_check(&client(), &server.url, &v("1.0.0"), false, AppMode::Install)
                .await
                .expect_err("非法清单应报错");
            assert!(err.to_string().contains("JSON"), "错误信息应可读: {err}");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_none_when_no_arch_payload() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_json(MANIFEST_ARM64_ONLY, 200);
            let result = do_check(&client(), &server.url, &v("1.0.0"), false, AppMode::Install)
                .await
                .expect("检查不应失败");
            assert!(result.is_none(), "当前架构无产物应为 None");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_none_when_same_version() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_json(MANIFEST_JSON, 200);
            let result = do_check(&client(), &server.url, &v("1.2.0"), false, AppMode::Install)
                .await
                .expect("检查不应失败");
            assert!(result.is_none(), "版本相等应为 None");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_downgrade_flag() {
        tauri::async_runtime::block_on(async {
            // 该测试需要两次请求，mock 服务器响应两次
            let server = MockServer::spawn_json_n(MANIFEST_JSON, 200, 2);
            // 清单 1.2.0 < current 2.0.0
            let none = do_check(&client(), &server.url, &v("2.0.0"), false, AppMode::Install)
                .await
                .expect("检查不应失败");
            assert!(none.is_none(), "默认不允许降级");
            let some = do_check(&client(), &server.url, &v("2.0.0"), true, AppMode::Install)
                .await
                .expect("检查不应失败")
                .expect("允许降级应有更新");
            assert_eq!(some.version, v("1.2.0"));
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_none_in_develop_mode() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_json(MANIFEST_JSON, 200);
            let result = do_check(&client(), &server.url, &v("1.0.0"), false, AppMode::Develop)
                .await
                .expect("检查不应失败");
            assert!(result.is_none(), "Develop 模式不更新");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_portable_prefers_portable() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_json(MANIFEST_JSON, 200);
            let info = do_check(&client(), &server.url, &v("1.0.0"), false, AppMode::Portable)
                .await
                .expect("检查不应失败")
                .expect("应有可用更新");
            assert_eq!(info.kind, UpdateKind::Portable, "Portable 模式优先 portable");
            assert_eq!(info.artifact.url.as_str(), "https://example.com/app-x64-portable.zip");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn select_artifact_rules() {
        let both = ArchPayloads {
            nsis: Some(artifact("n")),
            portable: Some(artifact("p")),
        };
        let only_nsis = ArchPayloads {
            nsis: Some(artifact("n")),
            portable: None,
        };
        let only_portable = ArchPayloads {
            nsis: None,
            portable: Some(artifact("p")),
        };
        let none = ArchPayloads {
            nsis: None,
            portable: None,
        };

        // Install：优先 nsis；无 nsis → None
        assert_eq!(
            select_artifact(&both, AppMode::Install),
            Some((UpdateKind::Nsis, artifact("n")))
        );
        assert_eq!(select_artifact(&only_portable, AppMode::Install), None);

        // Portable：优先 portable；无 portable 有 nsis → Nsis（可迁移为安装版）
        assert_eq!(
            select_artifact(&both, AppMode::Portable),
            Some((UpdateKind::Portable, artifact("p")))
        );
        assert_eq!(
            select_artifact(&only_nsis, AppMode::Portable),
            Some((UpdateKind::Nsis, artifact("n")))
        );
        assert_eq!(select_artifact(&none, AppMode::Portable), None);

        // Develop：不更新
        assert_eq!(select_artifact(&both, AppMode::Develop), None);
    }
}
