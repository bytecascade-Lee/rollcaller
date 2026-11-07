use crate::common::enums;
use crate::config::app_paths::{AppMode, Installer};
use crate::updater::constant::{DEFAULT_UPDATE_CHANNEL, DEFAULT_UPDATE_LEVEL};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
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
    #[serde(with = "crate::util::serde_utils::optional_timestamp_iso8601")]
    #[ts(type = "string | null")]
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
            AppMode::Portable => payloads.portable.clone(),
            AppMode::Install(installer) => payloads.get(installer),
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
    /// 开发环境直更载荷（debug exe，产物文件名见 `DEVELOP_UPDATE_BIN_NAME`）
    pub develop: Option<Artifact>,
}

impl Payloads {
    /// 按安装形态取载荷
    ///
    /// 清单里没有该形态的槽位时返回 `None` —— 即"该形态在本版本无产物"，
    /// 按契约视为无更新（不回退到 nsis）。
    fn get(&self, installer: Installer) -> Option<Artifact> {
        match installer {
            Installer::Nsis => self.nsis.clone(),
            // 清单暂无这些形态的槽位：取不到即「该版本无该形态产物」⇒ 无更新
            Installer::Msi | Installer::Dmg | Installer::Rpm | Installer::Deb | Installer::AppImage => None,
        }
    }
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
            .and_then(|mut segments| segments.next_back())
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

impl Default for Policy {
    fn default() -> Policy {
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
/// （[`crate::updater::state::DownloadSlot`]）的写入口与快照、以及 download 通道
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
                Some(info) => UpdateView::Available {
                    info,
                    severity: self.severity,
                },
                None => UpdateView::UpToDate,
            },
            UpdateStatus::Downloading => match info(&self.info) {
                Some(info) => UpdateView::Downloading { info },
                None => UpdateView::Checking,
            },
            UpdateStatus::Downloaded => match info(&self.info) {
                Some(info) => UpdateView::Downloaded {
                    info,
                    severity: self.severity,
                },
                None => UpdateView::UpToDate,
            },
            // `status == Error` 不变量保证 `error` 为 `Some`；兜底仅供防御（不应触发）
            UpdateStatus::Error => UpdateView::Error(
                self.error
                    .clone()
                    .unwrap_or(UpdateError::Check("未知错误，请重新执行检查".to_string())),
            ),
        }
    }
}

// `CNB` 是平台专有名称（cnb.cool），缩写不拆成 `Cnb`
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateSource {
    Github,
    CNB,
    Local,
}

impl fmt::Display for UpdateSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            UpdateSource::Github => "Github",
            UpdateSource::CNB => "CNB",
            UpdateSource::Local => "Local",
        };
        write!(f, "{}", s)
    }
}

/// 幅度门槛：最少需要多大的数字变化才触发更新。
///
/// 由 `UpdateLevel` 承载：`None` 表示禁止所有更新。
///
/// | 门槛 | 1.2.1→1.2.2 | 1.2.1→1.3.0 | 1.2.1→2.0.0 |
/// |---|---|---|---|
/// | Major | 跳过 | 跳过 | 更新 |
/// | Minor | 跳过 | 更新 | 更新 |
/// | Patch | 更新 | 更新 | 更新 |
/// | Never | 跳过 | 跳过 | 跳过 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateLevel {
    /// 仅主版本
    Major,
    /// 次版本及以上
    Minor,
    /// 补丁及以上
    Patch,
    /// 停止更新
    Never,
}

/// 发布渠道：决定版本列表的可见性。
///
/// - **Stable**：只接收稳定版。latest 为预发布版本时一律跳。
/// - **Prerelease**：接收预发布版本，且预发布相关路径有两条特殊规则：
///   - 「预发布 → 预发布」是**通道内递进**（如 1.2.0-rc.1 → 1.2.0-rc.2）：进入通道时风险已被接受，跟随通道、不再询问，**不适用幅度门槛**；
///   - 「预发布 → 稳定版」是**逃逸通道**（如 1.2.0-rc.1 → 1.2.0）：始终更新，不适用幅度门槛与通道，否则用户会永远卡在预发布版本上。
///
/// 而「稳定版 → 预发布」是**通道切换**（如 1.2.0 → 1.3.0-rc.1）：需要通道为 Prerelease才可见，且仍按幅度门槛判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateChannel {
    /// 正式版
    Stable,
    /// 预发布版
    Prerelease,
}

/// 更新严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// 普通更新：受用户设置的更新策略约束
    #[default]
    Normal,
    /// 重要更新：豁免用户的幅度门槛，通知所有用户
    Important,
    /// 紧急更新：豁免用户的幅度门槛并强制更新，如不升级则无法使用
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
pub enum UpdateDecision {
    /// 不更新
    Skip,
    /// 更新
    Update,
}

/// 安装形态，前端展示用（序列化为 `"nsis"` | `"portable"`，命令契约）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum UpdateKind {
    /// NSIS 安装包
    Nsis,
    /// 便携版 zip
    Portable,
}

/// 当前更新所处的阶段（对外快照的阶段标记）
///
/// 状态机由后端推进、以快照返回，前端 store 只做镜像。序列化为小写驼峰
/// （`"idle"` / `"checking"` / `"upToDate"` / `"available"` / `"downloading"` /
/// `"downloaded"` / `"error"`），供前端直接消费。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStatus {
    /// 初始：尚未检查
    #[default]
    Idle,
    /// 检查进行中（防重入：此阶段拒绝再次 check）
    Checking,
    /// 检查完，无可用更新
    UpToDate,
    /// 有可用更新（severity=critical 即强制更新，前端不应提供忽略/稍后）
    Available,
    /// 下载中（防重入：此阶段拒绝再次 check/download）
    Downloading,
    /// 已下载待安装
    Downloaded,
    /// 出错（恢复入口见 [`UpdateError`]，变体即"重试该调哪个命令"）
    Error,
}

/// 对外展示视图：命令返回值与广播的**统一裁剪契约**
///
/// 每个变体只携带该阶段前端真正需要渲染的字段；凭据（artifact、产物路径等）
/// 一律留在后端会话里，不出现于此。`status` 为 tag、载荷在 `data` 中，
/// 前端 store 对"命令返回"与"广播事件"用同一个类型与同一个 apply 逻辑。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "status", content = "data", rename_all = "camelCase")]
pub enum UpdateView {
    /// 空闲（尚未检查 / 无会话）
    Idle,
    /// 检查进行中
    Checking,
    /// 检查完，已是最新版
    UpToDate,
    /// 有可用更新（severity=critical 即强制更新，前端不应提供忽略/稍后）
    Available { info: UpdateInfo, severity: Severity },
    /// 下载中（纯状态：不带进度数字，实时进度经 download 通道窄帧推送）
    Downloading { info: UpdateInfo },
    /// 已下载待安装
    Downloaded { info: UpdateInfo, severity: Severity },
    /// 出错（[`UpdateError`] 的 `message` 供展示；`type` 供前端决定重试按钮对应的命令）
    Error(UpdateError),
}

/// 更新失败（带载荷的错误枚举，对外视图与后端会话直接携带）
///
/// 变体身份 = "失败发生在哪一步 / 重试该调哪个命令"，载荷 = 展示文案。
/// 序列化为 `{ "type": "check" | "download" | "install", "message": "..." }`，
/// 前端据 `type` 决定重试按钮对应的命令，`message` 展示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "message", rename_all = "camelCase")]
pub enum UpdateError {
    /// 检查失败 → 重试 check
    Check(String),
    /// 下载失败 → 重试 download（后端保留凭据）
    Download(String),
    /// 安装失败 → 重试 install（后端保留产物路径）
    Install(String),
}
