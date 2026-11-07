//! 安装领域：安装前准备 + 平台实现分派

mod common;
mod platform;

use crate::config::app_paths::AppMode;
use semver::Version;
use std::path::Path;

/// 安装前准备更新器（只有便携 / 直更需要；NSIS 安装包自带安装器）
pub async fn ensure_updater(mode: AppMode) -> anyhow::Result<()> {
    match mode {
        AppMode::Portable | AppMode::Develop => platform::ensure_updater().await,
        AppMode::Install(_) => Ok(()),
    }
}

/// 启动安装器：返回 `Ok` 表示安装器已接管，调用方应随即退出
pub fn launch(mode: AppMode, artifact_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    platform::launch(mode, artifact_path, from, to)
}
