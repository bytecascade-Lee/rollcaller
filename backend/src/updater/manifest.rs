//! 自定义更新清单：数据模型与解析逻辑
//!
//! 对应 `latest-expected.json` 的结构（契约 A），后续更新管线任务均依赖此结构。

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;
use url::Url;

/// 更新严重程度：只影响「是否可见/通知」，不绕过用户确认（`force` 才绕过确认）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// 普通更新（默认）：受用户设置的更新策略（幅度门槛/通道）约束
    #[default]
    Normal,
    /// 重要更新（如严重 bug 修复）：豁免用户的幅度门槛，通知所有用户
    Important,
    /// 紧急更新（如安全漏洞）：豁免用户的幅度门槛；唯一可配合 `force=true` 的档位
    Critical,
}

/// 自定义更新清单：描述一个可发布版本及其各平台载荷
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    /// 目标版本，如 "1.2.0"（非法版本会使整条清单解析失败）
    #[ts(type = "string")]
    pub version: Version,
    /// 发布说明（可选）
    pub release_notes: Option<String>,
    /// 发布日期（ISO 8601，可选；解析失败时置 None，不阻塞）
    #[serde(default)]
    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_iso_8601")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_optional_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub publish_date: Option<jiff::Timestamp>,
    /// 更新严重程度（可选，默认 normal）：normal 之外的版本豁免用户的幅度门槛（floor），
    /// 仅影响"是否可见/通知"，不绕过用户确认
    #[serde(default)]
    pub severity: Severity,
    /// 是否强制更新（可选，默认 false）：绕过用户确认，在会话结束后/下次启动前安装。
    /// 约束：仅 `severity=critical` 时合法，否则客户端应忽略或校验报错
    #[serde(default)]
    pub force: bool,
    /// 各平台载荷
    #[serde(default)]
    pub platforms: Platforms,
}

impl UpdateManifest {
    /// 按架构键（"x86_64" | "arm64"）获取对应平台载荷
    pub fn artifact_for(&self, arch: &str) -> Option<&Payloads> {
        let windows = self.platforms.windows.as_ref()?;
        match arch {
            "x86_64" => windows.x86_64.as_ref(),
            "arm64" => windows.arm64.as_ref(),
            _ => None,
        }
    }
}

/// 平台分组（预留 macos/linux，本任务只实现 windows）
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct Platforms {
    pub windows: Option<WindowsPlatforms>,
}

/// Windows 架构分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct WindowsPlatforms {
    pub x86_64: Option<Payloads>,
    pub arm64: Option<Payloads>,
}

/// 单架构下的载荷分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct Payloads {
    /// NSIS 安装包（无 WiX，只有 NSIS）
    pub nsis: Option<Artifact>,
    /// 便携版 zip
    pub portable: Option<Artifact>,
}

/// 单个下载产物
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
pub struct Artifact {
    #[ts(type = "string")]
    pub url: Url,
    /// 十六进制小写 sha256
    pub sha256: String,
    /// base64(minisign 签名文本)，即 .sig 文件全文
    pub signature: String,
    /// 字节数
    pub size: u64,
}

/// 当前架构键：`"x86_64"` | `"arm64"`（非 Windows 或未知架构为 None）
pub fn current_arch_key() -> Option<&'static str> {
    Option::from(crate::common::constant::sys::ARCH)
}

/// 当前平台键：`"windows-x86_64"` | `"windows-aarch64"`（旧格式用，任务 08）
pub fn current_platform_key() -> Option<&'static str> {
    Option::from(crate::common::constant::sys::OS_ARCH_COMPATIBLE_WITH_HISTORY)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合法 fixture：按 latest-expected.json 结构，size 为数字
    const VALID_FIXTURE: &str = r#"{
        "version": "1.2.0",
        "releaseNotes": "修复若干问题",
        "publishDate": "2026-08-01T00:00:00+08:00",
        "platforms": {
            "windows": {
                "x86_64": {
                    "nsis": {
                        "url": "https://example.com/rollcaller_1.2.0_x64-setup.exe",
                        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                        "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHRlc3Qgc2lnbmF0dXJl",
                        "size": 12345678
                    },
                    "portable": {
                        "url": "https://example.com/rollcaller_1.2.0_x64-portable.zip",
                        "sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef01234567",
                        "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHRlc3Qgc2lnbmF0dXJl",
                        "size": 23456789
                    }
                },
                "arm64": {
                    "nsis": {
                        "url": "https://example.com/rollcaller_1.2.0_arm64-setup.exe",
                        "sha256": "1111111111111111111111111111111111111111111111111111111111111111",
                        "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHRlc3Qgc2lnbmF0dXJl",
                        "size": 11111111
                    },
                    "portable": {
                        "url": "https://example.com/rollcaller_1.2.0_arm64-portable.zip",
                        "sha256": "2222222222222222222222222222222222222222222222222222222222222222",
                        "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHRlc3Qgc2lnbmF0dXJl",
                        "size": 22222222
                    }
                }
            }
        }
    }"#;

    fn parse_fixture() -> UpdateManifest {
        serde_json::from_str(VALID_FIXTURE).expect("合法 fixture 应解析成功")
    }

    #[test]
    fn test_parse_valid_fixture() {
        let manifest = parse_fixture();
        assert_eq!(manifest.version, Version::parse("1.2.0").unwrap());
        assert_eq!(manifest.release_notes.as_deref(), Some("修复若干问题"));
        let expected_date: jiff::Timestamp = "2026-08-01T00:00:00+08:00".parse().unwrap();
        assert_eq!(manifest.publish_date, Some(expected_date));

        let windows = manifest.platforms.windows.as_ref().expect("windows 平台应存在");
        let x64 = windows.x86_64.as_ref().expect("x86_64 载荷应存在");
        let nsis = x64.nsis.as_ref().expect("nsis 产物应存在");
        assert_eq!(nsis.url.as_str(), "https://example.com/rollcaller_1.2.0_x64-setup.exe");
        assert_eq!(
            nsis.sha256,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        assert_eq!(nsis.size, 12345678);
        let portable = x64.portable.as_ref().expect("portable 产物应存在");
        assert_eq!(portable.size, 23456789);

        let arm = windows.arm64.as_ref().expect("arm64 载荷应存在");
        assert_eq!(
            arm.nsis.as_ref().unwrap().url.as_str(),
            "https://example.com/rollcaller_1.2.0_arm64-setup.exe"
        );
        assert_eq!(arm.portable.as_ref().unwrap().size, 22222222);
    }

    #[test]
    fn test_missing_optional_fields() {
        let json = r#"{
            "version": "1.2.0",
            "platforms": {
                "windows": {
                    "x86_64": {
                        "nsis": {
                            "url": "https://example.com/app.exe",
                            "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                            "signature": "c2ln",
                            "size": 100
                        }
                    }
                }
            }
        }"#;
        let manifest: UpdateManifest = serde_json::from_str(json).expect("缺可选字段应解析成功");
        assert_eq!(manifest.release_notes, None);
        assert_eq!(manifest.publish_date, None);
        // 嵌套可选字段缺失 → None
        let x64 = manifest.platforms.windows.as_ref().unwrap().x86_64.as_ref().unwrap();
        assert!(x64.nsis.is_some());
        assert!(x64.portable.is_none());
        assert!(manifest.platforms.windows.as_ref().unwrap().arm64.is_none());
    }

    #[test]
    fn test_invalid_version_errors() {
        for bad in ["", "abc", "1.2"] {
            let json = format!(
                r#"{{"version": "{}", "platforms": {{"windows": {{"x86_64": {{"nsis": {{
                    "url": "https://example.com/app.exe",
                    "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                    "signature": "c2ln",
                    "size": 100
                }}}}}}}}}}"#,
                bad
            );
            assert!(
                serde_json::from_str::<UpdateManifest>(&json).is_err(),
                "version={:?} 应解析失败",
                bad
            );
        }
    }

    #[test]
    fn test_invalid_publish_date_becomes_none() {
        let json = r#"{
            "version": "1.2.0",
            "publishDate": "not-a-date",
            "platforms": {
                "windows": {
                    "x86_64": {
                        "nsis": {
                            "url": "https://example.com/app.exe",
                            "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                            "signature": "c2ln",
                            "size": 100
                        }
                    }
                }
            }
        }"#;
        let manifest: UpdateManifest = serde_json::from_str(json).expect("非法 publishDate 不应报错");
        assert_eq!(manifest.publish_date, None);
    }

    #[test]
    fn test_unknown_fields_ignored() {
        let json = r#"{
            "version": "1.2.0",
            "releaseNotes": "hello",
            "publishDate": "2026-08-01T00:00:00Z",
            "channel": "stable",
            "minVersion": "1.0.0",
            "installMode": "nsis",
            "platforms": {
                "windows": {
                    "x86_64": {
                        "nsis": {
                            "url": "https://example.com/app.exe",
                            "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                            "signature": "c2ln",
                            "size": 100,
                            "extra": { "foo": 1 }
                        }
                    }
                }
            }
        }"#;
        let manifest: UpdateManifest = serde_json::from_str(json).expect("未知字段应被忽略");
        assert_eq!(manifest.version, Version::parse("1.2.0").unwrap());
        assert_eq!(manifest.release_notes.as_deref(), Some("hello"));
        assert_eq!(
            manifest.publish_date,
            Some("2026-08-01T00:00:00Z".parse::<jiff::Timestamp>().unwrap())
        );
    }

    #[test]
    fn test_severity_and_force_defaults() {
        // 老清单没有 severity/force 字段 → 解析为默认值（normal / false），不破坏兼容
        let json = r#"{
            "version": "1.2.0",
            "platforms": {
                "windows": {
                    "x86_64": {
                        "nsis": {
                            "url": "https://example.com/app.exe",
                            "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                            "signature": "c2ln",
                            "size": 100
                        }
                    }
                }
            }
        }"#;
        let manifest: UpdateManifest = serde_json::from_str(json).expect("老清单应解析成功");
        assert_eq!(manifest.severity, Severity::Normal);
        assert!(!manifest.force);
    }

    #[test]
    fn test_severity_and_force_parsed() {
        let json = r#"{
            "version": "1.2.1",
            "severity": "critical",
            "force": true,
            "platforms": {
                "windows": {
                    "x86_64": {
                        "nsis": {
                            "url": "https://example.com/app.exe",
                            "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                            "signature": "c2ln",
                            "size": 100
                        }
                    }
                }
            }
        }"#;
        let manifest: UpdateManifest = serde_json::from_str(json).expect("新清单应解析成功");
        assert_eq!(manifest.severity, Severity::Critical);
        assert!(manifest.force);
        // 非 critical 配 force=true 仍能解析（校验交给上层，模型层不拦截）
        let json2 = json.replace("\"severity\": \"critical\"", "\"severity\": \"important\"");
        let manifest2: UpdateManifest = serde_json::from_str(&json2).unwrap();
        assert_eq!(manifest2.severity, Severity::Important);
        assert!(manifest2.force);
        // 非法 severity 值应解析失败（防止发布端拼错）
        let json3 = json.replace("\"severity\": \"critical\"", "\"severity\": \"fatal\"");
        assert!(serde_json::from_str::<UpdateManifest>(&json3).is_err());
    }

    #[test]
    fn test_artifact_for() {
        let manifest = parse_fixture();
        let x64 = manifest.artifact_for("x86_64").expect("x86_64 载荷应存在");
        assert!(x64.nsis.is_some() && x64.portable.is_some());
        let arm = manifest.artifact_for("arm64").expect("arm64 载荷应存在");
        assert!(arm.nsis.is_some() && arm.portable.is_some());
        assert!(manifest.artifact_for("riscv64").is_none());
        assert!(manifest.artifact_for("").is_none());

        // windows 分组缺失时返回 None
        let m: UpdateManifest = serde_json::from_str(r#"{"version": "1.2.0", "platforms": {}}"#).unwrap();
        assert!(m.artifact_for("x86_64").is_none());

        // platforms 整体缺失时返回 None
        let m2: UpdateManifest = serde_json::from_str(r#"{"version": "1.2.0"}"#).unwrap();
        assert!(m2.artifact_for("x86_64").is_none());
    }
}
