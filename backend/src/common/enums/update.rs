use crate::common::entity::update::UpdateInfo;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

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
    Available {
        info: UpdateInfo,
        severity: Severity,
    },
    /// 下载中（纯状态：不带进度数字，实时进度经 download 通道窄帧推送）
    Downloading {
        info: UpdateInfo,
    },
    /// 已下载待安装
    Downloaded {
        info: UpdateInfo,
        severity: Severity,
    },
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
