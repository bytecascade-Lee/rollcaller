//! 便携版安装执行
//!
//! # 流程：
//!
//! 1. 解压已下载并校验的 portable zip 到 [`paths::zip_staging`]（`temp/update/staging/portable-source-{目标版本}/`）；
//! 2. 组装 Go updater 的 config.json 写盘（落 [`paths::updater_config`]，与解压内容分居，
//!    避免被 updater 连同 source 一起复制进 target；
//! 3. 分离式 spawn `updater.exe`（`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`），传入 config.json 路径；
//! 4. spawn 成功后返回 `Ok(())`——由调用方执行退出前清理
//!    （`shutdown_hooks::run_all()`，如关闭数据库）后 `exit(0)`，
//!    由 updater 执行 wait → update → launch → (rollback)。
//!
//! # 约定
//!
//! - Go updater 不会自动解压 zip：`update.source` 必须是已解压目录；
//! - config 字段写全，默认值也显式给出；
//! - Windows 路径统一写成正斜杠，避免 JSON 转义；
//! - 日志毫秒时间戳命名。

use super::common;
use crate::config::app_paths;
use crate::config::app_paths::AppMode;
use crate::service::update::install::common::{extract_zip, find_updater};
use crate::service::update::paths;
use crate::util::path_utils;
use anyhow::anyhow;
use semver::Version;
use serde_json::json;
use std::path::Path;

/// 便携版完整安装编排：解压 zip → 组装 config → spawn updater（不 `exit`）
///
/// - `zip_path`：已下载并校验通过的 portable zip；
/// - `from` / `to`：当前版本与目标版本（用于命名 config 与日志，标识一次安装）。
///
/// 成功路径 = 更新器已分离式启动（返回 `Ok(())`），**本进程应随即退出**：调用方需
/// 先执行 `shutdown_hooks::run_all().await`（关闭数据库等）再 `std::process::exit(0)`；
/// 任一准备步骤或 spawn 失败返回 `Err`，进程保持存活以便上层提示重试
/// （失败路径不触发退出清理）。
#[cfg(target_os = "windows")]
pub fn install_portable(zip_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    // 1. 更新器：本地 cache 中取最新（由编排层 install 异步段先 ensure_updater 下载/校验，
    //    此处只需原样发现；ensure 幂等，已存在则直接跳过下载）
    let updater_exe = find_updater()
        .ok_or_else(|| anyhow!("未找到更新器（预期位于 cache/update 或 cache/update/bin 下）"))?;

    // 2. 当前进程路径，推导 exe 目录与用户数据目录
    let exe_path = path_utils::current_exe_clean()?;
    let target_dir = exe_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("无法获取当前可执行文件目录"))?;
    let data_dir = target_dir.join("data"); // Portable：用户数据全部在 exe 旁 data 下

    // 3. 解压 zip 到 temp/update/staging/portable-source-{目标版本} 清残留，返回实际 source 目录
    // config.json 落 temp/update/config/portable-config-{from}-to-{to}.json ，与解压内容分居
    let staging = paths::zip_staging(&AppMode::Portable, to);
    let source_dir = extract_zip(zip_path, &staging)?;

    // 4. 一次安装的毫秒时间戳与文件名
    let log_file = {
        let dir = app_paths::logs_dir().join("u");
        std::fs::create_dir_all(&dir).map_err(|e| anyhow!("创建更新日志目录失败（{}）：{e}", dir.display()))?;
        dir.join(format!("portable-update-{from}-to-{to}-{}.log", jiff::Timestamp::now().as_microsecond()))
    };
    let config_path = paths::updater_config(&AppMode::Portable, from, to);

    // 备份落点：temp/update/backup/portable-backup-{from}-to-{to}-{uuid前6位}。
    // 位于 data 树内，因而同时命中 backup.exclude 与 update.preserve（见 compose_config 注释）；
    // 目录不必预建，Go updater 复制前会自行 MkdirAll。
    let backup_dir = paths::backup(&AppMode::Portable, from, to);

    // 5. 组装并写入 config
    // 父目录 temp/update/config 由本处确保存在：fs::write 不创建父目录，而该目录不属
    // bootstrap 预建的应用目录（只建到 temp_dir 顶层），无人代建
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow!("创建更新配置目录失败（{}）：{e}", parent.display()))?;
    }
    let config = compose_config(
        std::process::id(),
        &source_dir,
        &target_dir,
        &data_dir,
        &backup_dir,
        &exe_path,
        &log_file,
    );
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
        .map_err(|e| anyhow!("写入更新配置失败（{}）：{e}", config_path.display()))?;

    // 6. 分离式 spawn updater（成功后由调用方执行退出清理并 exit(0)）
    common::spawn_updater(&updater_exe, &config_path)
}

/// 非 Windows 平台：便携版安装不可用
#[cfg(not(target_os = "windows"))]
pub fn install_portable(_zip_path: &Path, _from: &Version, _to: &Version) -> anyhow::Result<()> {
    anyhow::bail!("便携版安装仅支持 Windows")
}

/// 组装 Go updater 的完整 config JSON（config version 3；字段与默认值对齐 config.schema.json，写全）
///
/// - 路径统一转正斜杠（免 JSON 转义，schema 两者皆收）；
/// - `wait.pids` = 当前应用进程 PID（updater 等待本进程 `exit(0)` 退出）；version 3 只认
///   `pids`（数组），写 `pid` 会被 loader 拒绝；
/// - `preserve` / `backup.exclude` = 用户数据目录（替换 target 时保留 data）；
/// - `backup.location` = [`paths::backup`]（`temp/update/backup/portable-backup-{from}-to-{to}-{uuid前6位}`）。
///   该落点位于 `data` 树内（Portable 的 `temp_dir` = `exe_dir/data/temp`），因而**同时**命中
///   `backup.exclude` 与 `preserve` 两条 `data_dir` 条目 —— 前者使备份遍历在 `data` 处短路、
///   不会把备份写进自己正在遍历的源树，后者使清理阶段跳过整棵 `data`、备份不会刚做完就被删除。
///   Go updater 的 loader 正是按这两条放行"位于 target 内的备份落点"，运行时记一条非致命告警；
///   `cleanupOnSuccess: false` = 更新成功后保留备份，留作人工确认/回滚；
/// - `stayAlive: 0` = 启动新进程后更新器分离退出；
/// - `runtime.log.file` + `runtime.log.level.{console,file}`：version 3 的日志形态（路径与级别
///   均收在 `runtime.log` 下），两路级别显式写 `info`，与此前仅有单路 `logFile` 时的实际级别一致；
/// - `captureOutput: false`（分离启动下该字段本就不会被读），`captureFormat` / `captureToFile`
///   当前亦无实际作用，一并显式写出默认值——本函数的约定是「字段写全，默认值也显式给出」。
fn compose_config(
    pid: u32,
    source_dir: &Path,
    target_dir: &Path,
    data_dir: &Path,
    backup_dir: &Path,
    exe_path: &Path,
    log_file: &Path,
) -> serde_json::Value {
    json!({
        "version": 3,
        "runtime": {
            "headless": false,
            "log": {
                "file": path_utils::to_slash(log_file),
                "level": {
                    "console": "info",
                    "file": "info",
                },
            },
        },
        "wait": {
            "pids": [pid],
            "timeout": 10000,
            "forceKill": true,
            "interval": 500,
        },
        "update": {
            "source": path_utils::to_slash(source_dir),
            "target": path_utils::to_slash(target_dir),
            "preserve": [path_utils::to_slash(data_dir)],
            "cleanBeforeCopy": true,
            "backup": {
                "enabled": true,
                "location": path_utils::to_slash(backup_dir),
                "exclude": [path_utils::to_slash(data_dir)],
                "cleanupOnSuccess": false,
            },
        },
        "launch": {
            "execution": {
                "mode": "direct",
                "path": path_utils::to_slash(exe_path),
            },
            "context": {
                "workspace": path_utils::to_slash(target_dir),
                "args": [],
                "env": {},
            },
            "lifecycle": {
                "stayAlive": 0,
                "captureOutput": false,
                "captureFormat": "log",
                "captureToFile": true,
            },
        },
        "rollback": {
            "enabled": true,
            "fallbackExecutable": path_utils::to_slash(exe_path),
            "maxAttempts": 2,
        },
    })
}

