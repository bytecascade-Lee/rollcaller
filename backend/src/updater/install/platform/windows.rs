//! Windows 安装实现：NSIS / Portable / Develop。

mod develop;
mod nsis;
mod portable;
mod process;

use crate::config::app_paths::{AppMode, Installer};
use anyhow::anyhow;
use semver::Version;
use std::path::Path;

/// 按安装形态启动对应安装器
pub fn launch(mode: AppMode, artifact_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    match mode {
        AppMode::Install(Installer::Nsis) => nsis::install(artifact_path.to_path_buf(), nsis::NsisOptions::default()),
        AppMode::Install(installer) => Err(anyhow!("当前安装形态（{installer}）暂不支持自动更新")),
        AppMode::Portable => portable::install_portable(artifact_path, from, to),
        AppMode::Develop => develop::install_develop(artifact_path, from, to),
    }
}
