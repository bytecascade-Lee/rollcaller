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

/// 展示给用户的更新信息
#[derive(Debug, Clone, Serialize, TS)]
pub struct UpdateInfo {
    #[ts(type = "string")]
    pub version: Version,
    /// releaseNotes
    pub notes: Option<String>,
    /// publishDate（格式化后的字符串，供前端展示）
    pub date: Option<String>,
}

/// 一次更新检查的对外结果
///
/// - `NoUpdate`：无可用更新（无更高版本 / 策略未放行 / 当前形态无产物），不打扰；
/// - `Available`：有可用更新（normal / important），可提供"忽略 / 稍后"；
/// - `Mandatory`：必须更新（critical + force），前端不应提供忽略/稍后；
/// - `Error`：检查失败（网络 / 清单非法，具体原因见消息）。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", content = "data")]
pub enum CheckOutcome {
    /// 无可用更新
    NoUpdate,
    /// 有可用更新（普通 / 重要，可忽略）
    Available(UpdateInfo),
    /// 必须更新（强制），否则无法使用
    Mandatory(UpdateInfo),
    /// 检查失败（网络或清单非法），消息为具体原因
    Error(String),
}

/// 一次成功的检查命中：展示信息 + 下载凭据（updater 的内部结果）
///
/// 由 service 层拆包：展示信息（`info`）组对外结果 [`CheckOutcome`]
/// 并展示给用户；`artifact` / `severity` / `force` 与展示信息一并存入会话
/// （`PendingUpdate`），供 download 消费与复核、以及区分普通 / 强制更新。
pub struct FoundUpdate {
    /// 展示信息（version / notes / date）
    pub info: UpdateInfo,
    /// 严重程度（组装对外结果 / 下载复核依赖）
    pub severity: Severity,
    /// 是否强制（true 时对外结果应为 `Mandatory`）
    pub force: bool,
    /// 批准下载的产物（download 消费）
    pub artifact: Artifact,
}
