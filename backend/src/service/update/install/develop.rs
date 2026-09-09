//! Develop 直更（debug exe 单文件覆盖）执行
//!
//! # 流程
//!
//! 1. 解压已下载并校验的 develop zip 到 [`paths::develop_zip_staging`]
//!    （`temp/update/develop-source-{目标版本}/`），source 目录内是待覆盖的
//!    `rollcaller.exe`（debug 构建）；
//! 2. 组装 Go updater 的 config.json 写盘（落 [`paths::develop_config`]）：
//!    **不备份、不清空、不 preserve、不回滚**——target 是编译目录
//!    （Develop 下即 `backend/target/debug`），只允许 updater 把 source 内的
//!    `rollcaller.exe` 覆盖写入同名文件，**绝不能整目录清理**；
//! 3. 分离式 spawn `updater.exe`（`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`），
//!    传入 config.json 路径；
//! 4. spawn 成功后返回 `Ok(())`——由调用方执行退出前清理后 `exit(0)`，
//!    由 updater 执行 wait（旧进程退出）→ update（覆盖 exe）→ launch（重启新 exe）。
//!
//! # 运行前提
//!
//! Develop 形态的 exe 是 debug 构建（前端走 devUrl），重启后的新 exe 仍需
//! 前端 dev server 在线（由演练方负责先起前端）。

use super::common;
use crate::config::app_paths;
use crate::service::update::install::common::{extract_zip, find_updater};
use crate::service::update::paths;
use crate::util::path_utils;
use anyhow::anyhow;
use semver::Version;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Develop 直更完整编排：解压 zip → 组装 config → spawn updater（不 `exit`）
///
/// - `zip_path`：已下载并校验通过的 develop zip（内含 debug `rollcaller.exe`）；
/// - `from` / `to`：当前版本与目标版本（用于命名 config 与日志，标识一次安装）。
///
/// 成功路径 = 更新器已分离式启动（返回 `Ok(())`），**本进程应随即退出**：调用方需
/// 先执行 `shutdown_hooks::run_all().await` 再 `std::process::exit(0)`；
/// 任一准备步骤或 spawn 失败返回 `Err`，进程保持存活以便上层提示重试。
#[cfg(target_os = "windows")]
pub fn install_develop(zip_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    // 1. 更新器：本地 cache 中发现（编排层 install 已先 ensure_updater 幂等下载/校验）
    let updater_exe = find_updater()
        .ok_or_else(|| anyhow!("未找到更新器（预期位于 cache/update 或 cache/update/bin 下）"))?;

    // 2. 当前进程路径 → 目标目录（Develop 下即 backend/target/debug）
    let exe_path = path_utils::current_exe_clean()?;
    let target_dir = exe_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("无法获取当前可执行文件目录"))?;

    // 3. 解压 develop zip 到 temp/update/develop-source-{目标版本} 清残留，返回 source 目录
    // config.json 落 temp/update/develop-config-{from}-to-{to}.json ，与解压内容分居
    let staging = paths::develop_zip_staging(to);
    let source_dir = extract_zip(zip_path, &staging)?;

    // 4. 一次安装的毫秒时间戳与日志/config 文件名
    let log_file = {
        let dir = app_paths::logs_dir().join("u");
        std::fs::create_dir_all(&dir).map_err(|e| anyhow!("创建更新日志目录失败（{}）：{e}", dir.display()))?;
        dir.join(format!("develop-update-{from}-to-{to}-{}.log", jiff::Timestamp::now().as_microsecond()))
    };
    let config_path = paths::develop_config(from, to);

    // 5. 组装并写入 config（不备份 / 不清空 / 不回滚，target 为编译目录）
    let config = compose_config(std::process::id(), &source_dir, &target_dir, &exe_path, &log_file);
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
        .map_err(|e| anyhow!("写入更新配置失败（{}）：{e}", config_path.display()))?;

    // 6. 分离式 spawn updater（成功后由调用方执行退出清理并 exit(0)）
    common::spawn_updater(&updater_exe, &config_path)
}

/// 非 Windows 平台：Develop 直更不可用
#[cfg(not(target_os = "windows"))]
pub fn install_develop(_zip_path: &Path, _from: &Version, _to: &Version) -> anyhow::Result<()> {
    anyhow::bail!("Develop 直更仅支持 Windows")
}

/// 组装 develop 直更的完整 config JSON（字段与默认值对齐 config.schema.json；写全）
///
/// 与 portable 的关键差异：
/// - `update.cleanBeforeCopy = false`：**不清空 target**（编译目录，其余中间产物必须保留），
///   updater 仅把 source 内文件覆盖写入 target 同名文件；
/// - `update.backup.enabled = false` 且 `preserve = []`：不备份（target 为编译目录，整目录
///   备份无意义且巨大）；
/// - `rollback.enabled = false`：不回滚（下载产物已整体验签，覆盖前失败可重试）。
///
/// 其余（wait / launch / runtime）与 portable 一致：路径统一转正斜杠；
/// `wait.pid` = 当前应用进程 PID（updater 等待本进程退出后覆盖 exe）；
/// `launch.execution.path` = 目标 exe（覆盖后的新版本），workspace = 其目录。
fn compose_config(
    pid: u32,
    source_dir: &Path,
    target_dir: &Path,
    exe_path: &Path,
    log_file: &Path,
) -> serde_json::Value {
    json!({
        "version": 1,
        "runtime": {
            "headless": false,
            "logFile": path_utils::to_slash(log_file),
        },
        "wait": {
            "pid": pid,
            "timeout": 10000,
            "forceKill": true,
            "interval": 500,
        },
        "update": {
            "source": path_utils::to_slash(source_dir),
            "target": path_utils::to_slash(target_dir),
            "preserve": [],
            "cleanBeforeCopy": false,
            "backup": {
                "enabled": false,
            },
        },
        "launch": {
            "execution": {
                "mode": "direct",
                "path": path_utils::to_slash(exe_path),
            },
            "context": {
                "workspace": PathBuf::from(path_utils::to_slash(target_dir)).parent().unwrap().parent().unwrap(),
                "args": [],
                "env": {},
            },
            "lifecycle": {
                "stayAlive": 0,
                "captureOutput": false,
            },
        },
        "rollback": {
            "enabled": false,
        },
    })
}
