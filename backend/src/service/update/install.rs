//! 安装领域：安装器实现（nsis / portable）+ 统一启动分派与退出收尾
//!
//! # 分层
//!
//! - [`nsis`] / [`portable`]：两个安装器的具体实现（启动安装器，成功后由调用方收尾）；
//! - [`launch`]：按运行形态分派到对应安装器（领域内唯一的启动入口）；
//! - [`finish_and_exit`]：安装器接管后的统一收尾——执行退出前清理
//!   （`shutdown_hooks::run_all()`，如关闭数据库）并 `exit(0)`（永不返回）。
//!
//! 编排方（`service/update_service.rs::install_downloaded`）只需：
//! `launch(...)?` → `finish_and_exit().await`；失败路径不触发清理，进程存活可重试。

pub mod nsis;
pub mod portable;

use crate::config::app_paths::AppMode;
use anyhow::bail;
use semver::Version;
use std::path::Path;

/// 启动安装器（按运行形态分派；成功后安装器接管，调用方执行 [`finish_and_exit`]）
///
/// - `mode`：运行形态。`Portable` → Go updater 编排；`Install` → NSIS；
///   `Develop` 直接报错（不安装；预留后续"开发版模拟升级"空间，届时在此扩展）。
/// - `artifact_path`：已下载并校验的产物（Portable = zip；Install = nsis exe）；
/// - `from` / `to`：当前版本与目标版本（Portable 用于命名 config 与日志；NSIS 不需要）。
///
/// 成功 = 安装器已启动（返回 `Ok(())`），本进程应随即 [`finish_and_exit`]；
/// 任一环节失败返回 `Err`，进程保持存活、资源未清理，调用方可提示重试。
pub fn launch(mode: AppMode, artifact_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    match mode {
        AppMode::Portable => portable::install_portable(artifact_path, from, to),
        AppMode::Install => nsis::install(artifact_path.to_path_buf(), nsis::NsisOptions::default()),
        // 开发模式不更新：不触发任何安装。后期若支持开发版模拟升级，在此分支扩展
        AppMode::Develop => bail!("开发模式不更新"),
    }
}

/// 安装器接管后的统一收尾：执行退出前清理并退出进程（永不返回）
///
/// 只有"确认即将退出"才应调用（即 [`launch`] 成功之后）；失败路径进程继续存活，
/// 清理过的资源（如已关闭的数据库）不应被仍在运行的进程使用。
pub async fn finish_and_exit() -> ! {
    crate::shutdown_hooks::run_all().await;
    std::process::exit(0);
}
