//! 便携版安装执行
//!
//! # 流程：
//!
//! 1. 解压已下载并校验的 portable zip 到 `temp_dir()/update/{目标版本}/`；
//! 2. 组装 Go updater 的 config.json 写盘；
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

use crate::config::app_paths;
use crate::util::path_utils;
use anyhow::anyhow;
use semver::Version;
use serde_json::json;
use std::fs::File;
use std::path::{Path, PathBuf};

/// 便携版完整安装编排：解压 zip → 组装 config → spawn updater（不 `exit`）
///
/// - `zip_path`：已下载并校验通过的 portable zip（`downloaded_path`）；
/// - `from` / `to`：当前版本与目标版本（用于命名 config 与日志，标识一次安装）。
///
/// 成功路径 = 更新器已分离式启动（返回 `Ok(())`），**本进程应随即退出**：调用方需
/// 先执行 `shutdown_hooks::run_all().await`（关闭数据库等）再 `std::process::exit(0)`；
/// 任一准备步骤或 spawn 失败返回 `Err`，进程保持存活以便上层提示重试
/// （失败路径不触发退出清理）。
#[cfg(target_os = "windows")]
pub fn install_portable(zip_path: &Path, from: &Version, to: &Version) -> anyhow::Result<()> {
    // 1. 更新器：本地 cache 中取最新。便携版更新器必须有，自动下载属 download.rs 的任务
    let updater_exe = find_updater(app_paths::cache_dir())
        .ok_or_else(|| anyhow!("未找到更新器（预期位于 cache/update 或 cache/update/bin 下）"))?;

    // 2. 当前进程路径，推导 exe 目录与用户数据目录
    let exe_path = path_utils::current_exe_clean()?;
    let target_dir = exe_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("无法获取当前可执行文件目录"))?;
    let data_dir = target_dir.join("data"); // Portable：用户数据全部在 exe 旁 data 下

    // 3. 解压 zip 到 temp/update/{目标版本}
    // 清残留，返回实际 source 目录
    let work_dir = app_paths::temp_dir().join("update");
    let extract_dir = work_dir.join(to.to_string());
    let source_dir = extract_zip(zip_path, &extract_dir)?;

    // 4. 一次安装的毫秒时间戳与文件名
    let stem = format!("update-{from}-to-{to}-{}", jiff::Timestamp::now().as_microsecond());
    let log_file = {
        let dir = app_paths::logs_dir().join("u");
        std::fs::create_dir_all(&dir).map_err(|e| anyhow!("创建更新日志目录失败（{}）：{e}", dir.display()))?;
        dir.join(format!("{stem}.log"))
    };
    let config_path = work_dir.join(format!("{stem}.json"));

    // 5. 组装并写入 config
    let config = compose_config(std::process::id(), &source_dir, &target_dir, &data_dir, &exe_path, &log_file);
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
        .map_err(|e| anyhow!("写入更新配置失败（{}）：{e}", config_path.display()))?;

    // 6. 分离式 spawn updater（成功后由调用方执行退出清理并 exit(0)）
    spawn_updater(&updater_exe, &config_path)
}

/// 非 Windows 平台：便携版安装不可用
#[cfg(not(target_os = "windows"))]
pub fn install_portable(_zip_path: &Path, _from: &Version, _to: &Version) -> anyhow::Result<()> {
    anyhow::bail!("便携版安装仅支持 Windows")
}

/// 组装 Go updater 的完整 config JSON（字段与默认值对齐 config.schema.json；写全）
///
/// - 路径统一转正斜杠（免 JSON 转义，schema 两者皆收）；
/// - `wait.pid` = 当前应用进程 PID（updater 等待本进程 `exit(0)` 退出）；
/// - `preserve` / `backup.exclude` = 用户数据目录（替换 target 时保留 data）；
/// - `backup.location` 留空 = Go updater 自动生成（target 兄弟目录 + 时间戳）；
/// - `stayAlive: 0` = 启动新进程后更新器分离退出。
fn compose_config(
    pid: u32,
    source_dir: &Path,
    target_dir: &Path,
    data_dir: &Path,
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
            "preserve": [path_utils::to_slash(data_dir)],
            "cleanBeforeCopy": true,
            "backup": {
                "enabled": true,
                "location": "",
                "exclude": [path_utils::to_slash(data_dir)],
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
            },
        },
        "rollback": {
            "enabled": true,
            "fallbackExecutable": path_utils::to_slash(exe_path),
            "maxAttempts": 2,
        },
    })
}

/// 在 cache/update（优先 bin 子目录，其次根目录）下查找最新 updater-*.exe
///
/// 文件名形如 `updater-0.1.2-windows-x86_64.exe`，版本取文件名中首个可解析的 semver 段；
/// 多个存在时取版本最大者。
fn find_updater(cache_dir: &Path) -> Option<PathBuf> {
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
fn extract_zip(zip_path: &Path, dest: &Path) -> anyhow::Result<PathBuf> {
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
fn spawn_updater(exec_path: &Path, config_path: &Path) -> anyhow::Result<()> {
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
