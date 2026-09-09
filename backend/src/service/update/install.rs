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

mod develop;
mod nsis;
mod portable;
mod common;

use crate::config::app_paths::AppMode;
use anyhow::anyhow;
use semver::Version;
use std::fs::File;
use std::path::{Path, PathBuf};

/// 安装前确保更新器就绪（Portable / Develop 形态需要；Install 无更新器直接返回）
///
/// 挂接在编排层 `install` 的异步段：Portable / Develop 模式先下载 / 校验更新器到
/// `cache/update/bin/`（幂等，已有则跳过），再由各安装器的 `find_updater`
/// 原样发现；失败返回 `Err`（不触碰会话，由编排层落错误）。
pub async fn ensure_updater(mode: AppMode) -> anyhow::Result<()> {
    if mode == AppMode::Portable || mode == AppMode::Develop {
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
///   - `Portable` -> Go updater 编排（整目录替换）；
///   - `Develop` -> Go updater 编排的 develop 直更（单文件覆盖当前 exe，见 [`develop`]）
/// - `artifact_path`：已下载并校验的产物（Install = nsis exe；Portable = portable zip；
///   Develop = develop zip）
/// - `from` / `to`：当前版本与目标版本（Portable / Develop 用于命名 config 与日志；
///   NSIS 不需要，后期将调整相关策略，不再传递这两个字段）
///
/// # 返回
///
/// - `OK(())` -> 安装器已启动，本进程应随即 [`finish_and_exit`]
/// - `Err` -> 任一环节失败返回 `Err`，进程保持存活、资源未清理，调用方可提示重试
///
pub fn launch(mode: AppMode, artifact_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    match mode {
        AppMode::Install => nsis::install(artifact_path.to_path_buf(), nsis::NsisOptions::default()),
        AppMode::Portable => portable::install_portable(artifact_path, from, to),
        AppMode::Develop => develop::install_develop(artifact_path, from, to),
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

// ---------------------------------------------------------------------------
// Go updater 共享工具：portable（整目录替换）与 develop（单文件覆盖）两个
// 安装器共用的发现 / 解压 / spawn 原语，提升到安装领域根供子模块复用。
// ---------------------------------------------------------------------------

/// 在 cache/update（优先 bin 子目录，其次根目录）下查找最新 updater-*.exe
///
/// 文件名形如 `updater-0.1.2-windows-x86_64.exe`，版本取文件名中首个可解析的 semver 段；
/// 多个存在时取版本最大者。**代码自发现，不硬编码路径 / 版本**。
pub(super) fn find_updater(cache_dir: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    dirs.push(cache_dir.join("update").join("bin"));
    dirs.push(cache_dir.join("update"));
    let mut best: Option<(Version, PathBuf)> = None;
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(stem) = name.strip_prefix("updater").and_then(|s| s.strip_suffix(".exe")) else {
                continue;
            };
            let Some(ver) = stem.split('-').find_map(|seg| Version::parse(seg).ok()) else {
                continue;
            };
            if best.as_ref().is_none_or(|(bv, _)| ver > *bv) {
                best = Some((ver, path));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// 解压 zip 到 `dest`；若 zip 内是单一顶层目录则返回该目录（解包一层），否则返回 `dest`
///
/// 解压的路径穿越防护被移除，后续有这方面的话需要小心
pub(super) fn extract_zip(zip_path: &Path, dest: &Path) -> anyhow::Result<PathBuf> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| anyhow!("清理解压目录失败（{}）：{e}", dest.display()))?;
    }
    std::fs::create_dir_all(dest).map_err(|e| anyhow!("创建解压目录失败（{}）：{e}", dest.display()))?;

    let file = File::open(zip_path).map_err(|e| anyhow!("打开下载产物失败（{}）：{e}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| anyhow!("读取 zip 失败（{}）：{e}", zip_path.display()))?;
    archive.extract(dest).map_err(|e| anyhow!("解压更新包失败：{e}"))?;

    // 单顶层目录检测：仅一个条目且为目录 → source 指向它（剥掉外壳层）
    let top: Vec<PathBuf> = std::fs::read_dir(dest)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    if top.len() == 1 && top[0].is_dir() {
        Ok(top[0].clone())
    } else {
        Ok(dest.to_path_buf())
    }
}

/// 分离式 spawn updater.exe（`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`）
///
/// 分离式：updater 不随本进程退出而终止，独立完成更新流程，失败返回 `Err`。
#[cfg(target_os = "windows")]
pub(super) fn spawn_updater(exec_path: &Path, config_path: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    std::process::Command::new(exec_path)
        .arg(config_path)
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("spawn updater.exe 失败: {e}"))
}
