//! Windows 安装实现：NSIS / Portable / Develop。

mod develop;
mod nsis;
mod portable;
mod process;
mod updater;

use crate::config::app_paths::{AppMode, Installer};
use crate::updater::install::common;
use crate::updater::paths;
use anyhow::anyhow;
use semver::Version;
use std::path::Path;

/// 安装前准备 Go 更新器（便携 / 直更共用）
pub async fn ensure_updater() -> anyhow::Result<()> {
    updater::ensure().await
}

/// 按安装形态启动对应安装器
pub fn launch(mode: AppMode, artifact_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    match mode {
        AppMode::Install(Installer::Nsis) => nsis::install(artifact_path.to_path_buf(), nsis::NsisOptions::default()),
        AppMode::Install(installer) => Err(anyhow!("当前安装形态（{installer}）暂不支持自动更新")),
        AppMode::Portable => portable::install_portable(artifact_path, from, to),
        AppMode::Develop => develop::install_develop(artifact_path, from, to),
    }
}

/// 便携 / 直更共用的收尾：取锁定更新器 → 写 config → 分离式 spawn
///
/// `build` 拿到本次会话的日志路径（config 里的 `runtime.log.file`），由各形态补齐差异段。
fn start_updater(
    mode: AppMode,
    from: &Version,
    to: &Version,
    build: impl FnOnce(&Path) -> serde_json::Value,
) -> anyhow::Result<()> {
    let updater = updater::find().ok_or_else(|| anyhow!("锁定版本的更新器未就绪（预期位于 cache/update/bin）"))?;
    let log_file = paths::updater_log(&mode, from, to);
    let config_path = paths::updater_config(&mode, from, to);
    common::write_json(&config_path, &build(&log_file))?;
    process::spawn_updater(&updater, &config_path, &log_file)
}
