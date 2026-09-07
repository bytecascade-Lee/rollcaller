//! 便携版安装执行
//!
//! # 流程：
//!
//! 1. 解压已下载并校验的 portable zip 到 [`paths::portable_zip_staging`]（`temp/update/portable-source-{目标版本}/`）；
//! 2. 组装 Go updater 的 config.json 写盘（落 [`paths::portable_config`]，与解压内容分居，
//!    避免被 updater 连同 source 一起复制进 target）；
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

use crate::common::constant::sys::ARCH;
use crate::common::constant::update::PORTABLE_UPDATER_LATEST_MANIFEST_CNB;
use crate::common::enums::sys::Arch;
use crate::config::app_paths;
use crate::service::update::paths;
use crate::service::update::verify::verify_sha256;
use crate::state::http_client;
use crate::util::path_utils;
use anyhow::{anyhow, Context};
use semver::Version;
use serde_json::json;
use std::fs::File;
use std::path::{Path, PathBuf};
use url::Url;

/// 便携版更新器就绪（安装前 ensure；幂等）
///
/// 简单拉取 + 下载：**无进度、无取消、不涉状态机**。触发点 = Portable 安装前
/// （编排层 `install` 异步段），由 [`super::ensure_updater`] 按形态转发到此。
///
/// # 流程
///
/// 1. [`find_updater`] 在 cache 已发现更新器 → 直接返回（幂等，不重复下载）；
/// 2. 拉更新器仓库 latest manifest（与主源一致用 CNB，[`PORTABLE_UPDATER_LATEST_MANIFEST_CNB`]）；
/// 3. 更新器清单是**扁平结构**（`{version, publishDate, windows:{x86_64|arm64:{url,sha256,size}}}`，
///    无 signature / severity / platforms 包装），故不建新结构体，直接 `serde_json::Value`
///    按 `data["windows"][arch]` 索引。url 文件名须为 `updater-{semver}-windows-{arch}.exe`
///    （[`find_updater`] 按该命名解析版本号）。
///    注：本函数刻意不转 `Artifact` 走标准 verify 管线——落点是持久 cache（非 temp
///    packages）、且仅 sha256；**一旦更新器支持 minisign，本函数即废弃**，届时走
///    标准 Artifact + verify 管线（`verify_artifact_path` 有空签名语义，只填字段、零改动）。
/// 4. 下载到 [`paths::portable_updater_bin`]（url 最后一段），内存收齐后整体校验再落盘
///    ——磁盘上从不出现"未校验的正式产物"。
#[cfg(target_os = "windows")]
pub async fn ensure_updater() -> anyhow::Result<PathBuf> {
    // 1. 已存在任意版本 → 幂等返回
    if let Some(existing) = find_updater(app_paths::cache_dir()) {
        return Ok(existing);
    }

    // 2. 拉更新器最新清单（与主源一致的 CNB 源）
    let text = http_client::client()
        .get(PORTABLE_UPDATER_LATEST_MANIFEST_CNB)
        .header("Accept", "application/json")
        .send()
        .await
        .context(anyhow!("拉取更新器清单失败：网络错误"))?
        .error_for_status()
        .context(anyhow!("拉取更新器清单失败：服务器返回错误状态"))?
        .text()
        .await
        .context(anyhow!("拉取更新器清单失败：读取响应失败"))?;

    // 3. 按当前架构索引扁平清单
    let arch_key = match ARCH {
        Arch::X86_64 => "x86_64",
        Arch::Arm64 => "arm64",
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).context(anyhow!("更新器清单不是合法 JSON"))?;
    let entry = &value["data"]["windows"][arch_key];
    let url_str = entry["url"]
        .as_str()
        .context(anyhow!("更新器清单缺少 windows.{arch_key}.url"))?;
    let sha256 = entry["sha256"]
        .as_str()
        .context(anyhow!("更新器清单缺少 windows.{arch_key}.sha256"))?;

    let url = Url::parse(url_str).context(anyhow!("更新器下载地址不是合法 URL：{url_str}"))?;
    let file_name = url
        .path_segments()
        .and_then(|segs| segs.last())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| anyhow!("更新器下载地址缺少文件名：{url_str}"))?;
    let dest = paths::portable_updater_bin(file_name);

    // 4. 下载（体积小，内存收齐后整体校验再落盘）
    let bytes = http_client::download()
        .get(url.as_str())
        .send()
        .await
        .context(anyhow!("下载更新器失败：网络错误"))?
        .error_for_status()
        .context(anyhow!("下载更新器失败：服务器返回错误状态"))?
        .bytes()
        .await
        .context(anyhow!("下载更新器失败：读取响应失败"))?;
    if let Some(size) = entry["size"].as_u64() {
        if bytes.len() as u64 != size {
            anyhow::bail!("更新器大小与清单不符：期望 {size} 字节，实际 {} 字节", bytes.len());
        }
    }
    verify_sha256(&bytes, sha256).context(anyhow!("更新器校验失败"))?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("创建更新器目录失败（{}）：{e}", parent.display()))?;
    }
    std::fs::write(&dest, bytes).map_err(|e| anyhow!("写入更新器失败（{}）：{e}", dest.display()))?;
    Ok(dest)
}

/// 非 Windows 平台：便携版安装不可用，更新器亦无需下载
#[cfg(not(target_os = "windows"))]
pub async fn ensure_updater() -> anyhow::Result<PathBuf> {
    anyhow::bail!("便携版安装仅支持 Windows")
}

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
    let updater_exe = find_updater(app_paths::cache_dir())
        .ok_or_else(|| anyhow!("未找到更新器（预期位于 cache/update 或 cache/update/bin 下）"))?;

    // 2. 当前进程路径，推导 exe 目录与用户数据目录
    let exe_path = path_utils::current_exe_clean()?;
    let target_dir = exe_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("无法获取当前可执行文件目录"))?;
    let data_dir = target_dir.join("data"); // Portable：用户数据全部在 exe 旁 data 下

    // 3. 解压 zip 到 temp/update/portable-source-{目标版本} 清残留，返回实际 source 目录
    // config.json 落 temp/update/portable-config-{from}-to-{to}.json ，与解压内容分居
    let staging = paths::portable_zip_staging(to);
    let source_dir = extract_zip(zip_path, &staging)?;

    // 4. 一次安装的毫秒时间戳与文件名
    let log_file = {
        let dir = app_paths::logs_dir().join("u");
        std::fs::create_dir_all(&dir).map_err(|e| anyhow!("创建更新日志目录失败（{}）：{e}", dir.display()))?;
        dir.join(format!("portable-update-{from}-to-{to}-{}.log", jiff::Timestamp::now().as_microsecond()))
    };
    let config_path = paths::portable_config(from, to);

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
