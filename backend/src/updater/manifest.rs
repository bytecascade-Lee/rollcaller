//! 自定义更新清单：数据模型与解析逻辑
//!
//! 对应 `latest-expected.json` 的结构（契约 A），后续更新管线任务均依赖此结构。

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

/// 自定义更新清单：描述一个可发布版本及其各平台载荷
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    /// 目标版本，如 "1.2.0"（非法版本会使整条清单解析失败）
    pub version: Version,
    /// 发布说明（可选）
    pub release_notes: Option<String>,
    /// 发布日期（ISO 8601，可选；解析失败时置 None，不阻塞）
    #[serde(default, deserialize_with = "deserialize_publish_date")]
    pub publish_date: Option<jiff::Timestamp>,
    /// 各平台载荷
    #[serde(default)]
    pub platforms: Platforms,
}

impl UpdateManifest {
    /// 按架构键（"x86_64" | "arm64"，见 [`current_arch_key`]）获取对应平台载荷
    pub fn artifact_for(&self, arch: &str) -> Option<&ArchPayloads> {
        let windows = self.platforms.windows.as_ref()?;
        match arch {
            "x86_64" => windows.x86_64.as_ref(),
            "arm64" => windows.arm64.as_ref(),
            _ => None,
        }
    }
}

/// 平台分组（预留 macos/linux，本任务只实现 windows）
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Platforms {
    pub windows: Option<WindowsPlatforms>,
}

/// Windows 架构分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowsPlatforms {
    pub x86_64: Option<ArchPayloads>,
    pub arm64: Option<ArchPayloads>,
}

/// 单架构下的载荷分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct ArchPayloads {
    /// NSIS 安装包（无 WiX，只有 NSIS）
    pub nsis: Option<Artifact>,
    /// 便携版 zip
    pub portable: Option<Artifact>,
}

/// 单个下载产物
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Artifact {
    pub url: Url,
    /// 十六进制小写 sha256
    pub sha256: String,
    /// base64(minisign 签名文本)，即 .sig 文件全文
    pub signature: String,
    /// 字节数
    pub size: u64,
}

/// 容错反序列化：publishDate 解析失败 → None，不阻塞整条清单
fn deserialize_publish_date<'de, D>(deserializer: D) -> Result<Option<jiff::Timestamp>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw.and_then(|s| s.parse::<jiff::Timestamp>().ok()))
}

/// 由 OS 与 ARCH 推导当前架构键（纯函数，便于测试）
///
/// 注意：Rust 里 arm64 对应 `aarch64`，JSON 键名是 `arm64`。
fn arch_key_for(os: &str, arch: &str) -> Option<&'static str> {
    if os != "windows" {
        return None;
    }
    match arch {
        "x86_64" => Some("x86_64"),
        "aarch64" => Some("arm64"),
        _ => None,
    }
}

/// 由 OS 与 ARCH 推导旧格式平台键（任务 08 发布脚本用）
///
/// 注意：旧格式的 ARM 键使用 Rust 原生架构名 `aarch64`，而非 JSON 键名 `arm64`。
fn platform_key_for(os: &str, arch: &str) -> Option<&'static str> {
    if os != "windows" {
        return None;
    }
    match arch {
        "x86_64" => Some("windows-x86_64"),
        "aarch64" => Some("windows-aarch64"),
        _ => None,
    }
}

/// 当前架构键：`"x86_64"` | `"arm64"`（非 Windows 或未知架构为 None）
pub fn current_arch_key() -> Option<&'static str> {
    arch_key_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// 当前平台键：`"windows-x86_64"` | `"windows-aarch64"`（旧格式用，任务 08）
pub fn current_platform_key() -> Option<&'static str> {
    platform_key_for(std::env::consts::OS, std::env::consts::ARCH)
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
        assert_eq!(
            nsis.url.as_str(),
            "https://example.com/rollcaller_1.2.0_x64-setup.exe"
        );
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
        let x64 = manifest
            .platforms
            .windows
            .as_ref()
            .unwrap()
            .x86_64
            .as_ref()
            .unwrap();
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
        let manifest: UpdateManifest =
            serde_json::from_str(json).expect("非法 publishDate 不应报错");
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
    fn test_artifact_for() {
        let manifest = parse_fixture();
        let x64 = manifest.artifact_for("x86_64").expect("x86_64 载荷应存在");
        assert!(x64.nsis.is_some() && x64.portable.is_some());
        let arm = manifest.artifact_for("arm64").expect("arm64 载荷应存在");
        assert!(arm.nsis.is_some() && arm.portable.is_some());
        assert!(manifest.artifact_for("riscv64").is_none());
        assert!(manifest.artifact_for("").is_none());

        // windows 分组缺失时返回 None
        let m: UpdateManifest =
            serde_json::from_str(r#"{"version": "1.2.0", "platforms": {}}"#).unwrap();
        assert!(m.artifact_for("x86_64").is_none());

        // platforms 整体缺失时返回 None
        let m2: UpdateManifest = serde_json::from_str(r#"{"version": "1.2.0"}"#).unwrap();
        assert!(m2.artifact_for("x86_64").is_none());
    }

    #[test]
    fn test_arch_key_for() {
        assert_eq!(arch_key_for("windows", "aarch64"), Some("arm64"));
        assert_eq!(arch_key_for("windows", "x86_64"), Some("x86_64"));
        assert_eq!(arch_key_for("windows", "i686"), None);
        assert_eq!(arch_key_for("linux", "x86_64"), None);
        assert_eq!(arch_key_for("macos", "aarch64"), None);
    }

    #[test]
    fn test_platform_key_for() {
        assert_eq!(platform_key_for("windows", "x86_64"), Some("windows-x86_64"));
        assert_eq!(platform_key_for("windows", "aarch64"), Some("windows-aarch64"));
        assert_eq!(platform_key_for("linux", "x86_64"), None);
        assert_eq!(platform_key_for("windows", "i686"), None);
    }
}
