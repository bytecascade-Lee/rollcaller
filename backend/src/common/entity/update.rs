use crate::common::constant::update::{DEFAULT_UPDATE_CHANNEL, DEFAULT_UPDATE_LEVEL};
use crate::common::enums;
use crate::common::enums::update::{Severity, UpdateChannel, UpdateError, UpdateLevel, UpdateStatus, UpdateView};
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
            AppMode::Develop => payloads.develop.clone(),
            AppMode::Install => payloads.nsis.clone(),
            AppMode::Portable => payloads.portable.clone(),
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
    /// 开发环境 rollcaller.exe
    pub develop: Option<Artifact>,
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

impl Artifact {
    /// 从发布 url 提取产物正式文件名（url 最后一段）
    ///
    /// url 以 `/` 结尾或无文件名段时返回 `None`——清单不合法，由调用方在
    /// 进入下载 / 就绪探测前报错，不在公共方法内 panic。
    pub fn file_name(&self) -> Option<String> {
        self.url
            .path_segments()
            .and_then(|segments| segments.last())
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string())
    }
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
/// 由编排层拆包：展示信息（`info`）投影到快照展示；`artifact` / `severity`
/// 与展示信息一并写入后端会话（[`UpdateSession`]），供 download 消费与复核。
pub struct FoundUpdate {
    /// 展示信息（version / notes / date）
    pub info: UpdateInfo,
    /// 严重程度（组装对外结果 / 下载复核依赖；critical 即强制更新）
    pub severity: Severity,
    /// 批准下载的产物（download 消费）
    pub artifact: Artifact,
}

/// 下载进度的通用载体
///
/// 一份载荷多处复用：download 域每 chunk 的回调参数、state 原子槽
/// （[`crate::state::update::DownloadSlot`]）的写入口与快照、以及 download 通道
/// 的窄进度事件帧——三处的语义都是"瞬时进度"，与持久会话事实（info /
/// severity / artifact）解耦。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct DownloadProgress {
    /// 已下载字节数
    #[ts(type = "number")]
    pub downloaded: u64,
    /// 总字节数（来自 Content-Length，未知时为 None）
    #[ts(type = "number | null")]
    pub total: Option<u64>,
}

/// 后端权威会话（Tauri manage 注入，跨命令共享）
///
/// 持凭据（`artifact`）与展示所需事实，仅为后端内部状态，不 Serialize、不导出；
/// 对外只经 [`UpdateSession::view`] 投影为裁剪的 [`UpdateView`]。
/// 产物路径与当前版本基线**不入会话**：产物路径由凭据经 paths 现推（磁盘为事实源），
/// 基线版本由命令层从 `package_info` 每次传入。
/// 进度**不入会话**（瞬时态，见 [`DownloadProgress`]）：落在 state 的原子槽，仅供
/// download 通道窄帧消费，不与凭据等持久事实混存。
#[derive(Debug, Clone)]
pub struct UpdateSession {
    /// 当前所处阶段
    pub status: UpdateStatus,
    /// 目标更新信息（展示）
    pub info: Option<UpdateInfo>,
    /// 严重程度（下载复核 / 展示；critical 即强制）
    pub severity: Severity,
    /// 已批准下载的产物凭据（download 消费；check 命中后写入）
    pub artifact: Option<Artifact>,
    /// 错误（`status == Error` 时必 `Some`，与状态同锁写入）
    pub error: Option<UpdateError>,
}

impl Default for UpdateSession {
    fn default() -> Self {
        Self {
            status: UpdateStatus::Idle,
            info: None,
            severity: Severity::Normal,
            artifact: None,
            error: None,
        }
    }
}

impl UpdateSession {
    /// 投影对外展示视图（丢弃凭据，只带当前阶段必要的展示字段）
    pub fn view(&self) -> UpdateView {
        let info = |i: &Option<UpdateInfo>| i.clone();
        match self.status {
            UpdateStatus::Idle => UpdateView::Idle,
            UpdateStatus::Checking => UpdateView::Checking,
            UpdateStatus::UpToDate => UpdateView::UpToDate,
            UpdateStatus::Available => match info(&self.info) {
                Some(info) => UpdateView::Available { info, severity: self.severity },
                None => UpdateView::UpToDate,
            },
            UpdateStatus::Downloading => match info(&self.info) {
                Some(info) => UpdateView::Downloading { info },
                None => UpdateView::Checking,
            },
            UpdateStatus::Downloaded => match info(&self.info) {
                Some(info) => UpdateView::Downloaded { info, severity: self.severity },
                None => UpdateView::UpToDate,
            },
            // `status == Error` 不变量保证 `error` 为 `Some`；兜底仅供防御（不应触发）
            UpdateStatus::Error => UpdateView::Error(
                self.error.clone().unwrap_or(UpdateError::Check("未知错误，请重新执行检查".to_string())),
            ),
        }
    }
}
