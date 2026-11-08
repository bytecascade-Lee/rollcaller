use crate::common::enums;
use crate::config::app_paths::{AppMode, Installer};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;
use url::Url;

/// 发布清单
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
    /// 取指定平台 / 架构 / 运行形态的载荷
    pub fn get_artifact(&self, os: enums::sys::OS, arch: enums::sys::Arch, app_mode: AppMode) -> Option<Artifact> {
        self.platforms.get(os)?.get(arch)?.get(app_mode).cloned()
    }
}

/// 平台分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct ManifestPlatforms {
    pub windows: Option<ManifestArches>,
    pub macos: Option<ManifestArches>,
    pub linux: Option<ManifestArches>,
}

impl ManifestPlatforms {
    /// 取某个平台的架构分组
    pub fn get(&self, os: enums::sys::OS) -> Option<&ManifestArches> {
        match os {
            enums::sys::OS::Windows => self.windows.as_ref(),
            enums::sys::OS::MacOS => self.macos.as_ref(),
            enums::sys::OS::Linux => self.linux.as_ref(),
        }
    }
}

/// 架构分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct ManifestArches {
    pub x86_64: Option<Payloads>,
    pub arm64: Option<Payloads>,
}

impl ManifestArches {
    /// 取某个架构的载荷分组
    pub fn get(&self, arch: enums::sys::Arch) -> Option<&Payloads> {
        match arch {
            enums::sys::Arch::X86_64 => self.x86_64.as_ref(),
            enums::sys::Arch::Arm64 => self.arm64.as_ref(),
        }
    }
}

/// 单架构下的载荷分组
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(default)]
pub struct Payloads {
    pub msi: Option<Artifact>,
    pub nsis: Option<Artifact>,
    pub dmg: Option<Artifact>,
    pub rpm: Option<Artifact>,
    pub deb: Option<Artifact>,
    pub appimage: Option<Artifact>,
    pub portable: Option<Artifact>,
    pub develop: Option<Artifact>,
}

impl Payloads {
    /// 按运行形态取载荷
    pub fn get(&self, mode: AppMode) -> Option<&Artifact> {
        match mode {
            AppMode::Develop => self.develop.as_ref(),
            AppMode::Portable => self.portable.as_ref(),
            AppMode::Install(Installer::Msi) => self.msi.as_ref(),
            AppMode::Install(Installer::Nsis) => self.nsis.as_ref(),
            AppMode::Install(Installer::Dmg) => self.dmg.as_ref(),
            AppMode::Install(Installer::Rpm) => self.rpm.as_ref(),
            AppMode::Install(Installer::Deb) => self.deb.as_ref(),
            AppMode::Install(Installer::AppImage) => self.appimage.as_ref(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::enums::sys::{Arch, OS};
    use std::collections::BTreeSet;

    /// 三平台并存的一份清单：每个平台各带自己那几种载荷
    const MULTI_PLATFORM: &str = r#"{
      "version": "0.9.0",
      "platforms": {
        "windows": {
          "x86_64": {
            "nsis":     { "url": "https://example.com/win-x86_64-nsis.exe",      "sha256": "00", "signature": "s", "size": 1 },
            "portable": { "url": "https://example.com/win-x86_64-portable.zip",  "sha256": "00", "signature": "s", "size": 1 },
            "develop":  { "url": "https://example.com/win-x86_64-develop.zip",   "sha256": "00", "signature": "s", "size": 1 }
          },
          "arm64": {
            "nsis":     { "url": "https://example.com/win-arm64-nsis.exe",       "sha256": "00", "signature": "s", "size": 1 }
          }
        },
        "macos": {
          "arm64": {
            "dmg":      { "url": "https://example.com/mac-arm64.dmg",            "sha256": "00", "signature": "s", "size": 1 }
          }
        },
        "linux": {
          "x86_64": {
            "deb":      { "url": "https://example.com/linux-x86_64.deb",         "sha256": "00", "signature": "s", "size": 1 },
            "appimage": { "url": "https://example.com/linux-x86_64.AppImage",    "sha256": "00", "signature": "s", "size": 1 },
            "develop":  { "url": "https://example.com/linux-x86_64-develop.tar.gz", "sha256": "00", "signature": "s", "size": 1 }
          }
        }
      }
    }"#;

    fn parse(json: &str) -> UpdateManifest {
        serde_json::from_str(json).expect("清单应可解析")
    }

    fn artifact(name: &str) -> Artifact {
        Artifact {
            url: Url::parse(&format!("https://example.com/{name}")).expect("URL 应合法"),
            sha256: "00".repeat(32),
            signature: "s".to_string(),
            size: 1,
        }
    }

    /// 每个平台/架构取到的都是自己那一段，不串台
    #[test]
    fn picks_the_payload_of_the_requested_platform() {
        let manifest = parse(MULTI_PLATFORM);
        let cases = [
            (
                OS::Windows,
                Arch::X86_64,
                AppMode::Install(Installer::Nsis),
                "win-x86_64-nsis.exe",
            ),
            (OS::Windows, Arch::X86_64, AppMode::Portable, "win-x86_64-portable.zip"),
            (OS::Windows, Arch::X86_64, AppMode::Develop, "win-x86_64-develop.zip"),
            (
                OS::Windows,
                Arch::Arm64,
                AppMode::Install(Installer::Nsis),
                "win-arm64-nsis.exe",
            ),
            (OS::MacOS, Arch::Arm64, AppMode::Install(Installer::Dmg), "mac-arm64.dmg"),
            (OS::Linux, Arch::X86_64, AppMode::Install(Installer::Deb), "linux-x86_64.deb"),
            (
                OS::Linux,
                Arch::X86_64,
                AppMode::Install(Installer::AppImage),
                "linux-x86_64.AppImage",
            ),
            (OS::Linux, Arch::X86_64, AppMode::Develop, "linux-x86_64-develop.tar.gz"),
        ];
        for (os, arch, mode, expected) in cases {
            let label = format!("{os} / {arch} / {mode}");
            let found = manifest
                .get_artifact(os, arch, mode)
                .unwrap_or_else(|| panic!("{label} 应有产物"));
            assert_eq!(found.file_name().as_deref(), Some(expected), "{label}");
        }
    }

    /// 平台、架构、载荷任一缺失都返回 None，绝不回落到别的槽
    #[test]
    fn missing_platform_arch_or_payload_is_none() {
        let manifest = parse(MULTI_PLATFORM);
        assert!(manifest
            .get_artifact(OS::MacOS, Arch::X86_64, AppMode::Install(Installer::Dmg))
            .is_none());
        assert!(manifest
            .get_artifact(OS::Linux, Arch::Arm64, AppMode::Install(Installer::Deb))
            .is_none());
        assert!(manifest.get_artifact(OS::Windows, Arch::Arm64, AppMode::Develop).is_none());
        assert!(manifest
            .get_artifact(OS::Windows, Arch::X86_64, AppMode::Install(Installer::Msi))
            .is_none());
    }

    /// 老清单（只有 windows、甚至没有 platforms）照常解析
    #[test]
    fn windows_only_manifest_still_parses() {
        let manifest = parse(
            r#"{"version":"0.8.0","platforms":{"windows":{"x86_64":{"nsis":
               {"url":"https://example.com/win-nsis.exe","sha256":"00","signature":"s","size":1}}}}}"#,
        );
        assert!(manifest
            .get_artifact(OS::Windows, Arch::X86_64, AppMode::Install(Installer::Nsis))
            .is_some());
        assert!(manifest
            .get_artifact(OS::MacOS, Arch::Arm64, AppMode::Install(Installer::Dmg))
            .is_none());
        assert!(manifest.get_artifact(OS::Linux, Arch::X86_64, AppMode::Develop).is_none());

        let bare = parse(r#"{"version":"0.8.0"}"#);
        assert!(bare.get_artifact(OS::Windows, Arch::X86_64, AppMode::Develop).is_none());
    }

    /// 槽名 = `AppMode::slug()` = 发布端 `Kind.key`：改字段名却忘了改 slug、或加了槽却没接
    /// `get`，都会在这里挂。
    #[test]
    fn payload_slots_mirror_app_mode_slugs() {
        let payloads = Payloads {
            nsis: Some(artifact("nsis")),
            dmg: Some(artifact("dmg")),
            deb: Some(artifact("deb")),
            appimage: Some(artifact("appimage")),
            portable: Some(artifact("portable")),
            develop: Some(artifact("develop")),
        };
        let modes = [
            AppMode::Develop,
            AppMode::Portable,
            AppMode::Install(Installer::Nsis),
            AppMode::Install(Installer::Dmg),
            AppMode::Install(Installer::Deb),
            AppMode::Install(Installer::AppImage),
        ];

        let json = serde_json::to_value(&payloads).expect("载荷分组应可序列化");
        let keys: BTreeSet<&str> = json.as_object().expect("应为对象").keys().map(String::as_str).collect();
        let expected: BTreeSet<&str> = modes.iter().map(|mode| mode.slug()).collect();
        assert_eq!(keys, expected, "槽名必须与 AppMode::slug() 一致");

        for mode in modes {
            let found = payloads.get(mode).unwrap_or_else(|| panic!("{mode} 应有槽"));
            assert_eq!(found.file_name().as_deref(), Some(mode.slug()), "{mode}");
        }
        assert!(payloads.get(AppMode::Install(Installer::Msi)).is_none());
        assert!(payloads.get(AppMode::Install(Installer::Rpm)).is_none());
    }
}
