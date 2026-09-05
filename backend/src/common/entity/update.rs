use crate::common::constant::update::{DEFAULT_UPDATE_CHANNEL, DEFAULT_UPDATE_LEVEL};
use crate::common::enums;
use crate::common::enums::update::{Severity, UpdateChannel, UpdateLevel};
use crate::config::app_paths::AppMode;
use semver::Version;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use url::Url;

/// 自定义更新清单：描述一个可发布版本及其各平台载荷
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    #[ts(type = "string")]
    pub version: Version,
    pub release_notes: Option<String>,

    #[serde(default)]
    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_iso_8601")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_optional_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub publish_date: Option<jiff::Timestamp>,

    #[serde(default)]
    pub severity: Severity,

    #[serde(default)]
    pub force: bool,

    #[serde(default)]
    pub platforms: OS,
}

impl UpdateManifest {
    pub fn get_artifact(&self, os: enums::sys::OS, arch: enums::sys::Arch, app_mode: AppMode) -> Option<Artifact> {
        let arch_map = match os {
            enums::sys::OS::Windows => self.platforms.windows.as_ref()?,
            _ => return None,
        };
        let payloads = match arch {
            enums::sys::Arch::X86_64 => arch_map.x86_64.as_ref()?,
            enums::sys::Arch::Arm64 => arch_map.arm64.as_ref()?,
        };

        match app_mode {
            AppMode::Install => payloads.nsis.clone(),
            AppMode::Portable => payloads.portable.clone(),
            _ => None
        }
    }
}

/// 系统分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct OS {
    pub windows: Option<Arch>,
}

/// 架构分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct Arch {
    pub x86_64: Option<Payloads>,
    pub arm64: Option<Payloads>,
}

/// 单架构下的载荷分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct Payloads {
    /// NSIS 安装包
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

/// 用户更新策略，判定输入的统一载体
///
/// `level` 为 `None` 表示用户关闭了所有更新
/// `channel` 只在 `level` 非 `None` 时有意义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct Policy {
    /// 幅度门槛
    pub level: UpdateLevel,
    /// 发布渠道
    pub channel: UpdateChannel,
}

impl Policy {
    pub fn default() -> Policy {
        Policy {
            level: DEFAULT_UPDATE_LEVEL,
            channel: DEFAULT_UPDATE_CHANNEL,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
pub struct HistoryVersion {
    #[ts(type = "string")]
    pub version: Version,
    /// 缺省视为 normal（老索引未标定时兼容，语义与 manifest 的 severity 默认一致）
    #[serde(default)]
    pub severity: Severity,
    /// 缺省视为 false
    #[serde(default)]
    pub force: bool,
}

