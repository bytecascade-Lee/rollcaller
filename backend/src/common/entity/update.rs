use crate::common::constant::update::{DEFAULT_UPDATE_CHANNEL, DEFAULT_UPDATE_LEVEL};
use crate::common::enums;
use crate::common::enums::update::{Severity, UpdateChannel, UpdateErrorKind, UpdateLevel, UpdateStatus};
use crate::config::app_paths::AppMode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
#[ts(export)]
pub struct UpdateInfo {
    #[ts(type = "string")]
    pub version: Version,
    /// releaseNotes
    pub notes: Option<String>,
    /// publishDate（格式化后的字符串，供前端展示）
    pub date: Option<String>,
}

/// 一次成功的检查命中：展示信息 + 下载凭据（`service/update/check` 的内部结果）
///
/// 由编排层拆包：展示信息（`info`）投影到快照展示；`artifact` / `severity` /
/// `force` 与展示信息一并写入后端会话（[`UpdateSession`]），供 download 消费与复核。
pub struct FoundUpdate {
    /// 展示信息（version / notes / date）
    pub info: UpdateInfo,
    /// 严重程度（组装对外结果 / 下载复核依赖）
    pub severity: Severity,
    /// 是否强制（true 时对外结果应为强制更新）
    pub force: bool,
    /// 批准下载的产物（download 消费）
    pub artifact: Artifact,
}

/// 一次更新检查的对外快照（命令返回 / 前端 store 镜像的统一载体）
///
/// 后端 `state::update::UpdaterState` 是权威执行状态，内部另持下载凭据
/// （[`Artifact`]、产物路径等，见 [`UpdateSession`]）；本结构只含展示所需的
/// 简单数据，随命令返回值回传，前端据 `status` 渲染、按 `errorKind` 提供重试。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    /// 当前所处阶段
    pub status: UpdateStatus,
    /// 目标更新信息（`Available` / `Downloading` / `Downloaded` 等阶段存在）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<UpdateInfo>,
    /// 严重程度（有更新时才有意义）
    #[serde(default)]
    pub severity: Severity,
    /// 是否强制更新（force=true 时前端不应提供忽略/稍后）
    #[serde(default)]
    pub force: bool,
    /// 已下载字节数（`Downloading` 阶段，进度条用）
    #[ts(type = "number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloaded: Option<u64>,
    /// 总字节数（`Downloading` 阶段）
    #[ts(type = "number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// 错误消息（`status == Error` 时存在）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 错误来源（前端据此处决定"重试"按钮对应哪个命令）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<UpdateErrorKind>,
}

/// 后端权威会话（Tauri manage 注入，跨命令共享）
///
/// 与对外快照 [`UpdateState`] 的区别：额外持有不暴露给前端的下载凭据
/// `artifact`（url/sha256/签名）与 `current_version` / `downloaded_path`，
/// download 消费凭据、install 消费落盘产物；快照只是它的展示投影。
#[derive(Debug, Clone)]
pub struct UpdateSession {
    /// 当前所处阶段
    pub status: UpdateStatus,
    /// 目标更新信息（展示）
    pub info: Option<UpdateInfo>,
    /// 严重程度（下载复核 / 展示）
    pub severity: Severity,
    /// 是否强制（下载复核 / 展示）
    pub force: bool,
    /// 已批准下载的产物凭据（download 消费；check 命中后写入）
    pub artifact: Option<Artifact>,
    /// 判定时的基线版本（download 入口复核对照）
    pub current_version: Option<Version>,
    /// 已下载产物的落盘路径（install 消费）
    pub downloaded_path: Option<PathBuf>,
    /// 已下载字节数（`Downloading` 进度）
    pub downloaded: u64,
    /// 总字节数（`Downloading` 进度）
    pub total: Option<u64>,
    /// 错误消息
    pub error: Option<String>,
    /// 错误来源（重试入口）
    pub error_kind: Option<UpdateErrorKind>,
}

impl Default for UpdateSession {
    fn default() -> Self {
        Self {
            status: UpdateStatus::Idle,
            info: None,
            severity: Severity::Normal,
            force: false,
            artifact: None,
            current_version: None,
            downloaded_path: None,
            downloaded: 0,
            total: None,
            error: None,
            error_kind: None,
        }
    }
}

impl UpdateSession {
    /// 投影对外快照（丢弃内部凭据）
    pub fn snapshot(&self) -> UpdateState {
        UpdateState {
            status: self.status,
            info: self.info.clone(),
            severity: self.severity,
            force: self.force,
            downloaded: (self.status == UpdateStatus::Downloading).then_some(self.downloaded),
            total: (self.status == UpdateStatus::Downloading).then_some(self.total).flatten(),
            error: self.error.clone(),
            error_kind: self.error_kind,
        }
    }
}
