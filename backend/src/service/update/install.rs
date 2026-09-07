//! 安装领域：安装器实现（nsis / portable）+ 统一启动分派与退出收尾
//!
//! # 分层
//!
//! - [`nsis`] / [`portable`]：两个安装器的具体实现（启动安装器，成功后由调用方收尾）；
//! - [`launch`]：按运行形态分派到对应安装器（领域内唯一的启动入口）；
//! - [`finish_and_exit`]：安装器接管后的统一收尾——执行退出前清理
//!   （`shutdown_hooks::run_all()`，如关闭数据库）并 `exit(0)`（永不返回）。
//!
//! 编排方（`service/update.rs::install_update`）只需：
//! `launch(...)?` → `finish_and_exit().await`；失败路径不触发清理，进程存活可重试。

mod nsis;
mod portable;

use crate::config::app_paths::AppMode;
use semver::Version;
use std::path::Path;

/// 安装前确保更新器就绪（仅 Portable 形态需要；Install / Develop 无更新器直接返回）
///
/// 挂接在编排层 `install` 的异步段：Portable 模式先下载 / 校验更新器到
/// `cache/update/bin/`（幂等，已有则跳过），再由 install_portable 的
/// `find_updater` 原样发现；失败返回 `Err`（不触碰会话，由编排层落错误）。
pub async fn ensure_updater(mode: AppMode) -> anyhow::Result<()> {
    if mode == AppMode::Portable {
        portable::ensure_updater().await?;
    }
    Ok(())
}

/// 启动安装器（按运行形态分派；成功后安装器接管，调用方执行 [`finish_and_exit`]）
///
/// # 参数
///
/// - `mode`：运行形态。
///   - `Install` -> NSIS；
///   - `Portable` -> Go updater 编排
///   - `Develop` -> 暂时先和 `Install` 相同，走 NSIS
/// - `artifact_path`：已下载并校验的产物（Install = nsis exe；Portable = zip）
/// - `from` / `to`：当前版本与目标版本（Portable 用于命名 config 与日志；NSIS 不需要，后期将调整相关策略，不再传递这两个字段）
///
/// # 返回
///
/// - `OK(())` -> 安装器已启动，本进程应随即 [`finish_and_exit`]
/// - `Err` -> 任一环节失败返回 `Err`，进程保持存活、资源未清理，调用方可提示重试
///
pub fn launch(mode: AppMode, artifact_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    match mode {
        AppMode::Install | AppMode::Develop => nsis::install(artifact_path.to_path_buf(), nsis::NsisOptions::default()),
        AppMode::Portable => portable::install_portable(artifact_path, from, to),
    }
}

/// 安装器接管后的统一收尾：执行退出前清理并退出进程，永不返回
///
/// 只有"确认即将退出"才应调用（即 [`launch`] 成功之后）；失败路径进程继续存活，
/// 清理过的资源（如已关闭的数据库）不应被仍在运行的进程使用。
pub async fn finish_and_exit() -> ! {
    crate::shutdown_hooks::run_all().await;
    std::process::exit(0);
}
