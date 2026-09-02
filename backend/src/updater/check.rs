//! 更新检查：HTTP 拉取自定义清单 → severity/force 融合判定 → 返回更新信息
//!
//! 判定语义分层：
//! - 纯策略判定在 [`crate::updater::version_policy::decide`]（不认识 severity，保持
//!   纯粹"通道门禁 + 幅度门槛 + 降级开关"语义）；
//! - severity 豁免与 force 合法性在 [`evaluate`] 融合（上层职责）——本模块的
//!   `check` 与 download 入口复核共用同一个纯函数，判定语义只写一次。

use crate::config::app_paths::{current_mode, AppMode};
use crate::updater::manifest::{Artifact, Payloads, Severity, UpdateManifest};
use crate::updater::version_policy::{decide, Policy, UpdateChannel, UpdateDecision, UpdateLevel};
use anyhow::anyhow;
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 安装形态，前端展示用（序列化为 `"nsis"` | `"portable"`，命令契约）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum UpdateKind {
    /// NSIS 安装包
    Nsis,
    /// 便携版 zip
    Portable,
}

/// 更新信息（check 的返回值；经 Tauri 命令序列化透传给前端展示）
///
/// `severity` / `force` 为清单字段的透传：前端据此决定徽标与"是否提供忽略/稍后"
/// 的交互，真正的强制安装流程属于后续 force 任务。只做输出（Serialize + TS），
/// 不需要 Deserialize——实例由 check 内部组装。
#[derive(Debug, Clone, Serialize, TS)]
pub struct UpdateInfo {
    #[ts(type = "string")]
    pub version: Version,
    /// releaseNotes
    pub notes: Option<String>,
    /// publishDate（格式化后的字符串，供前端展示）
    pub date: Option<String>,
    /// 本次要下载的产物（nsis 或 portable 之一）
    pub artifact: Artifact,
    /// 原始清单，便于前端透传/调试
    #[ts(type = "any")]
    pub raw_json: serde_json::Value,
    /// 本次安装形态
    pub kind: UpdateKind,
    /// 更新严重程度（默认 normal；normal 之外的版本已豁免幅度门槛才会走到这里）
    pub severity: Severity,
    /// 是否强制（默认 false；仅 critical 合法）
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum DownloadSource {
    GITHUB,
    CNB,
}

pub const GITHUB: &str = "https://github.com/bytecascade-Lee/rollcaller/releases/latest/download/latest-github.json";
pub const CNB: &str = "https://cnb.cool/ordinary-glory/rollcaller/-/releases/latest/download/latest-cnb.json";

/// 更新批准判定结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approval {
    /// 不通知用户（策略不符 / 无需更新）
    Skip,
    /// 允许更新（含 severity 豁免后放行的场景）
    Approved,
}

/// 版本判定 + severity/force 融合（纯函数，check 与 download 复核共用）
///
/// # severity 豁免语义（收敛自策略文档与任务讨论）
///
/// | decide 返回 Skip 的原因 | normal | important / critical（force=false） | critical + force=true |
/// |---|---|---|---|
/// | 幅度不足（被 Minor/Major 门槛挡） | 不通知 | **豁免 → 通知** | 豁免 → 通知 |
/// | 用户关闭更新（level=None） | 不通知 | 不通知 | **穿透 → 通知** |
/// | Stable 通道拦预发布 | 不豁免 | 不豁免 | 不豁免 |
/// | 同版本 / 降级（allow_downgrade=false） | 不豁免 | 不豁免 | 不豁免 |
///
/// 实现方式：豁免 = 仅把幅度门槛临时降到 `Patch`（等价"无视幅度门槛"），其余
/// 门禁原样交给 `decide`——因此无需给 `decide` 增加 Skip 原因，也不破坏其纯
/// Copy 枚举形态。`force=true` 且 `severity != Critical` 视为清单非法（Err），
/// 宁可暴露发布端错误也不静默降级。
pub fn evaluate(
    current: &Version,
    latest: &Version,
    policy: &Policy,
    severity: Severity,
    force: bool,
) -> Result<Approval, String> {
    // 0. force 合法性：仅 critical 可强制（发布端错误，check 阶段直接报错）
    if force && severity != Severity::Critical {
        return Err(format!("清单不合法：force=true 仅允许 severity=critical（当前 {severity:?}）"));
    }
    // 1. 用户关闭更新（level=None）：normal / important / 非强制的 critical 均不打扰；
    //    唯一例外 = critical + force（发布方明确"必须更新"）
    if policy.level.is_none() {
        return Ok(if severity == Severity::Critical && force {
            Approval::Approved
        } else {
            Approval::Skip
        });
    }
    // 2. severity 豁免：normal 之外把幅度门槛降到 Patch，等价"无视幅度门槛"；
    //    通道门禁 / 同版本 / 降级 / 逃逸 / 递进由 decide 原样保留
    let level = if severity == Severity::Normal {
        policy.level
    } else {
        Some(UpdateLevel::Patch)
    };
    Ok(match decide(current, latest, level, policy.channel, policy.allow_downgrade) {
        UpdateDecision::Update => Approval::Approved,
        UpdateDecision::Skip => Approval::Skip,
    })
}

/// 检查是否有可用更新
///
/// `current_version` 由调用处从 `app.package_info().version` 解析后传入，
/// `policy` 为当前用户策略（设置存储落地前由调用方提供默认值），
/// 本函数保持纯函数风格，便于单元测试。
pub async fn check(
    client: &Client,
    endpoint: &str,
    current: &Version,
    policy: &Policy,
) -> anyhow::Result<Option<UpdateInfo>> {
    do_check(client, endpoint, current, policy, current_mode()).await
}

/// check 内部实现：运行模式由外部传入，便于测试非 Develop 路径
async fn do_check(
    client: &Client,
    endpoint: &str,
    current_version: &Version,
    policy: &Policy,
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
    let text = resp.text().await.map_err(|e| anyhow!("检查更新失败（读取响应失败）：{e}"))?;

    // 2. 解析清单（复用 manifest 类型）
    let raw_json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| anyhow!("检查更新失败（清单不是合法 JSON）：{e}"))?;
    let manifest: UpdateManifest =
        serde_json::from_value(raw_json.clone()).map_err(|e| anyhow!("检查更新失败（清单格式不符合规范）：{e}"))?;

    // 3. 当前架构无产物 → 无可用更新（不报错）
    let Some(arch) = Option::from(crate::common::constant::sys::ARCH) else {
        return Ok(None);
    };
    let Some(payloads) = manifest.artifact_for(arch) else {
        return Ok(None);
    };

    // 4. 按运行模式选择产物；无产物 → 无可用更新
    let Some((kind, artifact)) = select_artifact(payloads, mode) else {
        return Ok(None);
    };

    // 5. 判定：severity 豁免 + force 合法性在 evaluate（纯策略 decide 不认识 severity）；
    //    force=true 且非 critical → 清单非法，直接报错
    match evaluate(
        current_version,
        &manifest.version,
        policy,
        manifest.severity,
        manifest.force,
    )
    .map_err(|e| anyhow!("检查更新失败：{e}"))?
    {
        Approval::Skip => Ok(None),
        Approval::Approved => Ok(Some(UpdateInfo {
            version: manifest.version.clone(),
            notes: manifest.release_notes.clone(),
            date: manifest.publish_date.map(|d| d.to_string()),
            artifact,
            raw_json,
            kind,
            severity: manifest.severity,
            force: manifest.force,
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
fn select_artifact(payloads: &Payloads, mode: AppMode) -> Option<(UpdateKind, Artifact)> {
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

        fn spawn_bytes_n(body: Vec<u8>, status: u16, content_type: Option<&'static str>, n: usize) -> Self {
            let server = Server::http("127.0.0.1:0").expect("mock 服务器启动失败");
            let port = server.server_addr().to_ip().expect("无法获取端口").port();
            let url = format!("http://127.0.0.1:{port}/latest.json");
            let handle = std::thread::spawn(move || {
                let mut served = 0;
                while served < n {
                    match server.recv_timeout(Duration::from_secs(10)) {
                        Ok(Some(request)) => {
                            let mut response = Response::from_data(body.clone()).with_status_code(StatusCode(status));
                            if let Some(ct) = content_type {
                                response =
                                    response.with_header(Header::from_bytes("Content-Type", ct).expect("header 合法"));
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

    /// 测试策略：默认 Patch + Stable，可调 level 与降级开关
    fn policy(level: Option<UpdateLevel>, allow_downgrade: bool) -> Policy {
        Policy {
            level,
            channel: UpdateChannel::Stable,
            allow_downgrade,
        }
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

    /// 动态生成单产物（x86_64 nsis）清单，便于构造 severity / force 组合
    fn severe_json(version: &str, severity: &str, force: bool) -> String {
        format!(
            r#"{{
                "version": "{version}",
                "severity": "{severity}",
                "force": {force},
                "platforms": {{
                    "windows": {{
                        "x86_64": {{
                            "nsis": {{
                                "url": "https://example.com/app-x64-setup.exe",
                                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                                "signature": "c2ln",
                                "size": 100
                            }}
                        }}
                    }}
                }}
            }}"#
        )
    }

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
            let info = do_check(
                &client(),
                &server.url,
                &v("1.0.0"),
                &policy(Some(UpdateLevel::Patch), false),
                AppMode::Install,
            )
            .await
            .expect("检查不应失败")
            .expect("应有可用更新");
            assert_eq!(info.version, v("1.2.0"));
            assert_eq!(info.notes.as_deref(), Some("修复若干问题"));
            assert!(info.date.is_some(), "publishDate 应格式化输出");
            assert_eq!(info.kind, UpdateKind::Nsis, "Install 模式应选 nsis");
            assert_eq!(info.artifact.url.as_str(), "https://example.com/app-x64-setup.exe");
            assert_eq!(info.raw_json["version"], "1.2.0", "应透传原始清单");
            // severity / force 透传：清单缺省 → normal / false
            assert_eq!(info.severity, Severity::Normal);
            assert!(!info.force);
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_errors_on_http_error() {
        tauri::async_runtime::block_on(async {
            let server = MockServer::spawn_json("", 404);
            let err = do_check(
                &client(),
                &server.url,
                &v("1.0.0"),
                &Policy::default(),
                AppMode::Install,
            )
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
            let err = do_check(
                &client(),
                &server.url,
                &v("1.0.0"),
                &Policy::default(),
                AppMode::Install,
            )
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
            let err = do_check(
                &client(),
                &server.url,
                &v("1.0.0"),
                &Policy::default(),
                AppMode::Install,
            )
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
            let result = do_check(
                &client(),
                &server.url,
                &v("1.0.0"),
                &Policy::default(),
                AppMode::Install,
            )
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
            let result = do_check(
                &client(),
                &server.url,
                &v("1.2.0"),
                &Policy::default(),
                AppMode::Install,
            )
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
            let none = do_check(
                &client(),
                &server.url,
                &v("2.0.0"),
                &policy(Some(UpdateLevel::Patch), false),
                AppMode::Install,
            )
            .await
            .expect("检查不应失败");
            assert!(none.is_none(), "默认不允许降级");
            let some = do_check(
                &client(),
                &server.url,
                &v("2.0.0"),
                &policy(Some(UpdateLevel::Patch), true),
                AppMode::Install,
            )
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
            let result = do_check(
                &client(),
                &server.url,
                &v("1.0.0"),
                &Policy::default(),
                AppMode::Develop,
            )
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
            let info = do_check(
                &client(),
                &server.url,
                &v("1.0.0"),
                &Policy::default(),
                AppMode::Portable,
            )
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
        let both = Payloads {
            nsis: Some(artifact("n")),
            portable: Some(artifact("p")),
        };
        let only_nsis = Payloads {
            nsis: Some(artifact("n")),
            portable: None,
        };
        let only_portable = Payloads {
            nsis: None,
            portable: Some(artifact("p")),
        };
        let none = Payloads {
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

    // ---------- evaluate 纯函数：severity 豁免表 ----------

    #[test]
    fn evaluate_exempts_floor_only_for_non_normal() {
        // Minor 门槛下的补丁差（1.2.0 → 1.2.1）
        let minor = policy(Some(UpdateLevel::Minor), false);
        assert_eq!(
            evaluate(&v("1.2.0"), &v("1.2.1"), &minor, Severity::Normal, false),
            Ok(Approval::Skip),
            "normal 补丁差被 Minor 门槛挡"
        );
        assert_eq!(
            evaluate(&v("1.2.0"), &v("1.2.1"), &minor, Severity::Important, false),
            Ok(Approval::Approved),
            "important 豁免幅度门槛"
        );
        assert_eq!(
            evaluate(&v("1.2.0"), &v("1.2.1"), &minor, Severity::Critical, false),
            Ok(Approval::Approved),
            "critical 豁免幅度门槛"
        );
        // 次版本差本来就达标：normal 也放行（豁免分支不干扰达标升级）
        assert_eq!(
            evaluate(&v("1.2.0"), &v("1.3.0"), &minor, Severity::Normal, false),
            Ok(Approval::Approved)
        );
    }

    #[test]
    fn evaluate_disabled_level_only_critical_force_passes() {
        // 用户关闭更新（level=None）：一律不打扰，唯一例外 = critical + force
        let off = policy(None, false);
        assert_eq!(evaluate(&v("1.2.0"), &v("9.9.9"), &off, Severity::Normal, false), Ok(Approval::Skip));
        assert_eq!(evaluate(&v("1.2.0"), &v("9.9.9"), &off, Severity::Important, false), Ok(Approval::Skip));
        assert_eq!(evaluate(&v("1.2.0"), &v("9.9.9"), &off, Severity::Critical, false), Ok(Approval::Skip));
        assert_eq!(evaluate(&v("1.2.0"), &v("9.9.9"), &off, Severity::Critical, true), Ok(Approval::Approved));
    }

    #[test]
    fn evaluate_force_requires_critical() {
        let p = Policy::default();
        assert!(evaluate(&v("1.2.0"), &v("1.2.1"), &p, Severity::Important, true).is_err());
        assert!(evaluate(&v("1.2.0"), &v("1.2.1"), &p, Severity::Normal, true).is_err());
        assert_eq!(
            evaluate(&v("1.2.0"), &v("1.2.1"), &p, Severity::Critical, true),
            Ok(Approval::Approved)
        );
    }

    #[test]
    fn evaluate_never_bypasses_channel_same_or_downgrade() {
        let p = Policy::default(); // Stable + 不允许降级
        // 稳定通道拦预发布：critical + force 也不豁免
        assert_eq!(
            evaluate(&v("1.2.0"), &v("1.3.0-rc.1"), &p, Severity::Critical, true),
            Ok(Approval::Skip)
        );
        // 同版本
        assert_eq!(evaluate(&v("1.2.0"), &v("1.2.0"), &p, Severity::Critical, true), Ok(Approval::Skip));
        // 降级：severity/force 不额外放行，完全由 allow_downgrade 决定
        assert_eq!(evaluate(&v("2.0.0"), &v("1.2.0"), &p, Severity::Critical, true), Ok(Approval::Skip));
        let allow_dl = Policy {
            allow_downgrade: true,
            ..Policy::default()
        };
        assert_eq!(evaluate(&v("2.0.0"), &v("1.2.0"), &allow_dl, Severity::Critical, true), Ok(Approval::Approved));
        assert_eq!(evaluate(&v("2.0.0"), &v("1.2.0"), &allow_dl, Severity::Normal, false), Ok(Approval::Approved));
    }

    // ---------- do_check 集成：severity / force 生效 ----------

    #[test]
    fn check_severity_exempts_floor() {
        tauri::async_runtime::block_on(async {
            // level=Minor + 补丁差（1.2.0 → 1.2.1）：normal 不通知，important 豁免放行
            let server_normal = MockServer::spawn_bytes_n(
                severe_json("1.2.1", "normal", false).into_bytes(),
                200,
                Some("application/json"),
                1,
            );
            let normal = do_check(
                &client(),
                &server_normal.url,
                &v("1.2.0"),
                &policy(Some(UpdateLevel::Minor), false),
                AppMode::Install,
            )
            .await
            .expect("检查不应失败");
            assert!(normal.is_none(), "normal 补丁差被 Minor 门槛挡");
            server_normal.handle.join().unwrap();

            let server_sev = MockServer::spawn_bytes_n(
                severe_json("1.2.1", "important", false).into_bytes(),
                200,
                Some("application/json"),
                1,
            );
            let info = do_check(
                &client(),
                &server_sev.url,
                &v("1.2.0"),
                &policy(Some(UpdateLevel::Minor), false),
                AppMode::Install,
            )
            .await
            .expect("检查不应失败")
            .expect("important 应豁免幅度门槛");
            assert_eq!(info.version, v("1.2.1"));
            assert_eq!(info.severity, Severity::Important);
            assert!(!info.force);
            server_sev.handle.join().unwrap();
        });
    }

    #[test]
    fn check_force_invalid_errors() {
        tauri::async_runtime::block_on(async {
            // force=true 且 severity=important → 清单非法，check 报错
            let server = MockServer::spawn_bytes_n(
                severe_json("1.2.1", "important", true).into_bytes(),
                200,
                Some("application/json"),
                1,
            );
            let err = do_check(
                &client(),
                &server.url,
                &v("1.2.0"),
                &Policy::default(),
                AppMode::Install,
            )
            .await
            .expect_err("force 与 severity 不匹配应报错");
            assert!(err.to_string().contains("清单不合法"), "错误信息应可读: {err}");
            server.handle.join().unwrap();
        });
    }

    #[test]
    fn check_critical_force_overrides_disabled_updates() {
        tauri::async_runtime::block_on(async {
            // 用户关闭更新（level=None）时，critical + force 仍穿透通知
            let server = MockServer::spawn_bytes_n(
                severe_json("1.2.1", "critical", true).into_bytes(),
                200,
                Some("application/json"),
                1,
            );
            let info = do_check(
                &client(),
                &server.url,
                &v("1.2.0"),
                &policy(None, false),
                AppMode::Install,
            )
            .await
            .expect("检查不应失败")
            .expect("critical+force 应穿透关闭开关");
            assert_eq!(info.version, v("1.2.1"));
            assert_eq!(info.severity, Severity::Critical);
            assert!(info.force, "force 应透传");
            server.handle.join().unwrap();
        });
    }
}
