use crate::common::enums;
use crate::config::app_paths::{AppMode, Installer};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;
use url::Url;

/// 发布清单：一个版本及其各平台载荷（`latest-{github|cnb|develop}.json`）
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
    pub platforms: ManifestPlatforms,
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

/// 平台分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct ManifestPlatforms {
    pub windows: Option<ManifestArches>,
}

/// 架构分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct ManifestArches {
    pub x86_64: Option<Payloads>,
    pub arm64: Option<Payloads>,
}

/// 单架构下的载荷分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct Payloads {
    pub nsis: Option<Artifact>,
    pub portable: Option<Artifact>,
    /// 开发环境直更载荷
    pub develop: Option<Artifact>,
}

impl Payloads {
    /// 根据安装形态取载荷
    fn get(&self, installer: Installer) -> Option<Artifact> {
        match installer {
            Installer::Nsis => self.nsis.clone(),
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
    /// base64 编码后的 signature
    pub signature: String,
    /// 字节数
    pub size: u64,
}

impl Artifact {
    /// 产物正式文件名
    pub fn file_name(&self) -> Option<String> {
        self.url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string())
    }
}

/// 用户更新策略
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
            level: UpdateLevel::Patch,
            channel: UpdateChannel::Stable,
        }
    }
}

/// 版本索引条目
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
pub struct HistoryVersion {
    #[ts(type = "string")]
    pub version: Version,
    /// 默认为 normal
    #[serde(default)]
    pub severity: Severity,
    /// 各源清单摘要
    #[serde(default)]
    pub manifests: ManifestDigests,
}

/// 单个源清单的摘要
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
pub struct ManifestDigest {
    /// 十六进制小写 sha256
    pub sha256: String,
    /// base64 编码后的 signature
    #[serde(default)]
    pub signature: Option<String>,
}

/// 三个源各自的清单摘要
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct ManifestDigests {
    pub github: Option<ManifestDigest>,
    pub cnb: Option<ManifestDigest>,
    pub develop: Option<ManifestDigest>,
}

impl ManifestDigests {
    /// 取某个源的摘要（`Local` 对应 `develop`，与 [`UpdateSource::slug`] 一致）
    pub fn get(&self, source: UpdateSource) -> Option<&ManifestDigest> {
        match source {
            UpdateSource::GitHub => self.github.as_ref(),
            UpdateSource::CNB => self.cnb.as_ref(),
            UpdateSource::Local => self.develop.as_ref(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.github.is_none() && self.cnb.is_none() && self.develop.is_none()
    }
}

/// 展示给用户的更新信息
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct UpdateInfo {
    #[ts(type = "string")]
    pub version: Version,
    /// releaseNotes
    pub notes: Option<String>,
    /// publishDate
    pub date: Option<String>,
}

/// 一次成功的检查命中
pub struct FoundUpdate {
    pub info: UpdateInfo,
    pub severity: Severity,
    pub artifact: Artifact,
}

/// 下载进度的通用载体
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct DownloadProgress {
    /// 已下载字节数
    #[ts(type = "number")]
    pub downloaded: u64,
    /// 总字节数
    #[ts(type = "number | null")]
    pub total: Option<u64>,
}

/// 后端权威更新会话
#[derive(Debug, Clone)]
pub struct UpdateSession {
    pub status: UpdateStatus,
    pub info: Option<UpdateInfo>,
    pub severity: Severity,
    /// 已批准下载的产物凭据
    pub artifact: Option<Artifact>,
    /// `status == Error` 时必为 `Some`，与状态同锁写入
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
    /// 投影对外视图
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
            // `status == Error` 不变量保证 `error` 为 `Some`
            UpdateStatus::Error => UpdateView::Error(
                self.error
                    .clone()
                    .unwrap_or(UpdateError::Check("未知错误，请重新执行检查".to_string())),
            ),
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateSource {
    GitHub,
    CNB,
    Local,
}

impl fmt::Display for UpdateSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            UpdateSource::GitHub => "GitHub",
            UpdateSource::CNB => "CNB",
            UpdateSource::Local => "Local",
        };
        write!(f, "{}", s)
    }
}

impl UpdateSource {
    /// 更新源的小写标识
    pub fn slug(&self) -> &'static str {
        match self {
            UpdateSource::GitHub => "github",
            UpdateSource::CNB => "cnb",
            UpdateSource::Local => "develop",
        }
    }

    /// 该源的清单是否必须带签名
    ///
    /// 发布源由 CI 签名，`just bump` 会拦住缺签名的历史版本；本地联调源跑在开发机上、
    /// 拿不到私钥，只能给摘要——这是唯一的例外。
    pub fn requires_manifest_signature(&self) -> bool {
        !matches!(self, UpdateSource::Local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 签名豁免只允许本地联调源一家
    #[test]
    fn only_local_source_may_skip_manifest_signature() {
        assert!(UpdateSource::GitHub.requires_manifest_signature());
        assert!(UpdateSource::CNB.requires_manifest_signature());
        assert!(!UpdateSource::Local.requires_manifest_signature());
    }
}

/// 幅度门槛，最少需要多大的数字变化才触发更新
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

/// 发布渠道：决定预发布版本是否可见
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
    /// 普通：受用户策略约束
    #[default]
    Normal,
    /// 重要：豁免幅度门槛
    Important,
    /// 紧急：无条件豁免门槛并强制更新
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
pub enum UpdateDecision {
    Skip,
    Update,
}

/// 当前更新所处的阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    /// 检查中，防重入
    Checking,
    UpToDate,
    /// 有可用更新
    Available,
    /// 下载中，防重入
    Downloading,
    Downloaded,
    /// 出错，变体即"重试该调哪个命令"
    Error,
}

/// 对外展示视图
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "status", content = "data", rename_all = "camelCase")]
pub enum UpdateView {
    /// 空闲：尚未检查 / 无会话
    Idle,
    /// 正在检查
    Checking,
    /// 已是最新版
    UpToDate,
    /// 有可用更新，severity=critical 即强制更新，前端不应提供忽略/稍后
    Available { info: UpdateInfo, severity: Severity },
    /// 下载中，不带进度数字，实时进度经 download 通道窄帧推送
    Downloading { info: UpdateInfo },
    /// 已下载，待安装
    Downloaded { info: UpdateInfo, severity: Severity },
    /// 出错，[`UpdateError`] 的 `message` 供展示；`type` 供前端决定重试按钮对应的命令
    Error(UpdateError),
}

/// 更新失败
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "message", rename_all = "camelCase")]
pub enum UpdateError {
    /// 检查失败 -> 重试 check
    Check(String),
    /// 下载失败 -> 重试 download，后端保留凭据
    Download(String),
    /// 安装失败 -> 重试 install，后端保留产物路径
    Install(String),
}
