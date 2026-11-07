//! 非 Windows 平台兜底：安装器与更新器都尚未实现。

use crate::config::app_paths::AppMode;
use semver::Version;
use std::path::Path;

pub async fn ensure_updater() -> anyhow::Result<()> {
    anyhow::bail!("自动更新器在 {} 上尚未实现", std::env::consts::OS)
}

pub fn launch(mode: AppMode, _artifact_path: &Path, _from: &Version, _to: &Version) -> anyhow::Result<()> {
    anyhow::bail!("{mode} 在 {} 上尚未实现自动更新", std::env::consts::OS)
}
