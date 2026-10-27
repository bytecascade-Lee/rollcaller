//! 安装领域：安装器实现（nsis / portable）+ 统一启动分派
//!
//! # 分层
//!
//! - [`nsis`] / [`portable`]：两个安装器的具体实现（启动安装器，成功后由调用方收尾）；
//! - [`launch`]：按运行形态分派到对应安装器（领域内唯一的启动入口）。
//!
//! 退出收尾不在这里，属于进程级语义：[`crate::shutdown_hooks::finish_and_exit`]。
//! 编排方（`service/update.rs::install_update`）只需：
//! `launch(...)?` → `shutdown_hooks::finish_and_exit(0).await`；失败路径不触发清理，
//! 进程存活可重试。

mod develop;
mod nsis;
mod portable;
mod common;

use crate::config::app_paths::AppMode;
use semver::Version;
use std::path::Path;

/// 安装前确保更新器就绪（Portable / Develop 形态需要；Install 无更新器直接返回）
///
/// 挂接在编排层 `install` 的异步段：Portable / Develop 模式先按**锁定版本**把更新器
/// 准备到 `cache/update/bin/`（缓存里就是这一版且校验通过则跳过），再由各安装器的
/// `updater_exe` 原样取用；失败返回 `Err`（不触碰会话，由编排层落错误）。
pub async fn ensure_updater(mode: AppMode) -> anyhow::Result<()> {
    if mode == AppMode::Portable || mode == AppMode::Develop {
        common::ensure_updater().await?;
    }
    Ok(())
}

/// 启动安装器（按运行形态分派；成功后安装器接管，调用方执行
/// [`crate::shutdown_hooks::finish_and_exit`]）
///
/// # 参数
///
/// - `mode`：运行形态。
///   - `Install` -> NSIS；
///   - `Portable` -> Go updater 编排（整目录替换）；
///   - `Develop` -> Go updater 编排的 develop 直更（单文件覆盖当前 exe，见 [`develop`]）
/// - `artifact_path`：已下载并校验的产物（Install = nsis exe；Portable = portable zip；
///   Develop = develop zip）
/// - `from` / `to`：当前版本与目标版本（Portable / Develop 用于命名 config 与日志；
///   NSIS 不需要，后期将调整相关策略，不再传递这两个字段）
///
/// # 返回
///
/// - `OK(())` -> 安装器已启动，本进程应随即调用
///   [`crate::shutdown_hooks::finish_and_exit`]
/// - `Err` -> 任一环节失败返回 `Err`，进程保持存活、资源未清理，调用方可提示重试
///
pub fn launch(mode: AppMode, artifact_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    match mode {
        AppMode::Install => nsis::install(artifact_path.to_path_buf(), nsis::NsisOptions::default()),
        AppMode::Portable => portable::install_portable(artifact_path, from, to),
        AppMode::Develop => develop::install_develop(artifact_path, from, to),
    }
}
