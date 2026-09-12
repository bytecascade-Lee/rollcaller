//! Develop 直更（debug exe 单文件写入）执行
//!
//! # 流程
//!
//! 1. 解压已下载并校验的 develop zip 到 [`paths::zip_staging`]
//!    （`temp/update/staging/develop-source-{目标版本}/`），source 目录内是待写入的
//!    debug exe（文件名见 [`DEVELOP_UPDATE_BIN_NAME`]）；
//! 2. 组装 Go updater 的 config.json 写盘（落 [`paths::updater_config`]）：
//!    **不备份、不清空、不 preserve、不回滚**——target 是编译目录
//!    （Develop 下即 `backend/target/debug`），只允许 updater 把 source 内那个 exe
//!    写入 target 同名文件，**绝不能整目录清理**；
//! 3. 分离式 spawn `updater.exe`（`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`），
//!    传入 config.json 路径；
//! 4. spawn 成功后返回 `Ok(())`——由调用方执行退出前清理后 `exit(0)`，
//!    由 updater 执行 wait（旧进程退出）→ update（写入）→ launch（**分离**启动新 exe，
//!    updater 随即退出；分离的原因见 `compose_config` 中 `stayAlive = 0` 的说明）。
//!
//! # 注意
//!
//! Develop 形态的 exe 是 debug 构建，但是已同步捆绑前端 dist 资源
//! 不需要再启动前端开发服务

use super::common;
use crate::common::constant::update::DEVELOP_UPDATE_BIN_NAME;
use crate::config::app_paths;
use crate::config::app_paths::AppMode;
use crate::service::update::install::common::{extract_zip, find_updater};
use crate::service::update::paths;
use crate::util::path_utils;
use anyhow::anyhow;
use semver::Version;
use serde_json::json;
use std::path::Path;

/// Develop 直更完整编排：解压 zip → 组装 config → spawn updater（不 `exit`）
///
/// - `zip_path`：已下载并校验通过的 develop zip（内含 debug exe，文件名见 [`DEVELOP_UPDATE_BIN_NAME`]）；
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
    // 启动目标 = 更新产物落点（独立文件名），**不是当前 exe**：详见 DEVELOP_UPDATE_BIN_NAME
    let launch_path = target_dir.join(DEVELOP_UPDATE_BIN_NAME);

    // 3. 解压 develop zip 到 temp/update/staging/develop-source-{目标版本} 清残留，返回 source 目录
    // config.json 落 temp/update/config/develop-config-{from}-to-{to}.json ，与解压内容分居
    let staging = paths::zip_staging(&AppMode::Develop, to);
    let source_dir = extract_zip(zip_path, &staging)?;

    // 4. 一次安装的毫秒时间戳与日志/config 文件名
    let log_file = {
        let dir = app_paths::logs_dir().join("u");
        std::fs::create_dir_all(&dir).map_err(|e| anyhow!("创建更新日志目录失败（{}）：{e}", dir.display()))?;
        dir.join(format!("develop-update-{from}-to-{to}-{}.log", jiff::Timestamp::now().as_microsecond()))
    };
    let config_path = paths::updater_config(&AppMode::Develop, from, to);

    // 5. 组装并写入 config（不备份 / 不清空 / 不回滚，target 为编译目录）
    // 父目录 temp/update/config 由本处确保存在：fs::write 不创建父目录，而该目录不属
    // bootstrap 预建的应用目录（只建到 temp_dir 顶层），无人代建
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow!("创建更新配置目录失败（{}）：{e}", parent.display()))?;
    }
    let config = compose_config(std::process::id(), &source_dir, &target_dir, &launch_path, &log_file);
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

/// 组装 develop 直更的完整 config JSON（config version 3；字段与默认值对齐 config.schema.json，写全）
///
/// 与 portable 的关键差异：
/// - `update.cleanBeforeCopy = false`：**不清空 target**（编译目录，其余中间产物必须保留），
///   updater 仅把 source 内文件覆盖写入 target 同名文件；
/// - `update.backup.enabled = false` 且 `preserve = []`：不备份（target 为编译目录，整目录
///   备份无意义且巨大），备份子对象的其余字段随之不写；
/// - `rollback.enabled = false`：不回滚（下载产物已整体验签，覆盖前失败可重试）。
///
/// `launch.lifecycle` 取 `stayAlive = 0`（分离启动，updater 随即退出），**这也是让新实例
/// 能活下来的关键**：Go updater 在 `stayAlive == 0` 时以 `DETACHED_PROCESS | CREATE_NO_WINDOW`
/// 启动新进程，新进程不继承任何控制台。若沿用驻留模式（`-1`），新进程会继承 updater 的控制台
/// ——而 updater 在 `headless = false` 下会 `AttachConsole` 附加回本进程的控制台（IDE run
/// session 那个），于是一旦 IDE 收尾关闭控制台，新进程即收到 CTRL_CLOSE_EVENT、以
/// `STATUS_CONTROL_C_EXIT (0xC000013A)` 静默死亡（实测复现）。分离后即免疫。
/// 代价：新进程无控制台、stdout 落 NUL 设备（要它的输出可直接手动运行该 exe，
/// 或看 debug 构建的 `logs/f` 文件日志——注意文件层级别为 WARN）。
/// `captureOutput = false`：`stayAlive == 0` 时该字段本就不会被读（updater 在分离分支直接
/// 返回），置 false 仅为表意；且捕获会把子进程输出以**嵌套形态**混进 updater 自己的日志，不宜开启。
/// `captureFormat` / `captureToFile` 随之无实际作用，显式写出默认值以贯彻本函数「字段写全」的约定。
///
/// 其余（wait / launch / runtime）与 portable 一致：路径统一转正斜杠；
/// `wait.pids` = 当前应用进程 PID（updater 等待本进程退出后再写入；version 3 只认 `pids`
/// 数组，写 `pid` 会被 loader 拒绝）；
/// `runtime.log.file` + `runtime.log.level.{console,file}` = version 3 的日志形态，两路级别显式写
/// `info`；
/// `launch.execution.path` = `launch_path`（更新产物落点，见 [`DEVELOP_UPDATE_BIN_NAME`]），
/// `launch.context.workspace` = target_dir 的上两级（cargo workspace 根）。
fn compose_config(
    pid: u32,
    source_dir: &Path,
    target_dir: &Path,
    launch_path: &Path,
    log_file: &Path,
) -> serde_json::Value {
    // launch.context.workspace = target_dir 的上两级（Develop 下 target_dir = backend/target/debug，
    // 上两级即 cargo workspace 根 backend）。Develop 属开发形态，祖先不足两级（如 exe 落在盘根）
    // 属布局错误：此处允许 panic 而不做静默兜底，但把实际 target_dir 一并打进 panic 文案以便定位。
    let workspace = target_dir.parent().and_then(Path::parent).unwrap_or_else(|| {
        panic!(
            "develop 直更要求 target_dir 至少有两级父目录以推导 workspace，当前 target_dir = {}",
            target_dir.display()
        )
    });

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
            "preserve": [],
            "cleanBeforeCopy": false,
            "backup": {
                "enabled": false,
            },
        },
        "launch": {
            "execution": {
                "mode": "direct",
                "path": path_utils::to_slash(launch_path),
            },
            "context": {
                "workspace": path_utils::to_slash(workspace),
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
            "enabled": false,
        },
    })
}
