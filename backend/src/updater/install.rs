//! 安装领域：平台无关的安装前准备 + 平台实现分派。
//!
//! 平台差异只出现在 `platform`（唯一的 cfg 站点）；`launch` 成功后安装器接管，调用方随即执行
//! [`crate::shutdown_hooks::finish_and_exit`]，失败则进程存活可重试。

mod common;
mod platform;

use crate::config::app_paths::AppMode;
use semver::Version;
use std::path::Path;

/// 安装前准备更新器（Portable / Develop 需要；Install 直接返回）
pub async fn ensure_updater(mode: AppMode) -> anyhow::Result<()> {
    if matches!(mode, AppMode::Portable | AppMode::Develop) {
        common::ensure_updater().await?;
    }
    Ok(())
}

/// 启动安装器（成功后安装器接管）
pub fn launch(mode: AppMode, artifact_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    platform::launch(mode, artifact_path, from, to)
}
