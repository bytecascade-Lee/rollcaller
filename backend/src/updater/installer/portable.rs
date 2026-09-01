//! 便携版安装执行（Go updater 编排，Windows 专用）
//!
//! 流程（任务 06 规格 2.3）：
//!
//! 1. 把已验证的下载字节（zip，内含整个应用目录）解压到暂存目录 `<staging>/<version>/`；
//! 2. 生成 Go updater 的 `config.json`（字段与 `D:\Go\updater\internal\config\schema.go` 对齐）；
//! 3. 分离式 spawn `updater.exe`（`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`）；
//! 4. spawn 成功 → 执行退出前钩子 → `exit(0)`，由 updater 执行 wait → update → launch → (rollback)。
//!
//! **退出码契约**（与 Go updater 对齐，见其 README「退出码」节）：
//! `0` 成功 / `2` wait 失败 / `3` update 失败 / `4` launch 失败（含回滚成功）/ `5` rollback 失败。
//! Rust 侧只做编排，不重新实现 wait/update/launch/rollback。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// 便携版更新工具路径（占位，由仓库作者补；应指向随应用分发的 updater.exe）
const UPDATER_EXE: &str = "<便携版更新工具路径，仓库作者补>";
/// 更新暂存目录（占位，由仓库作者补）；实际解压位置为其下的 `<version>/`
const UPDATER_STAGING_DIR: &str = "<更新暂存目录，仓库作者补>";
/// 备份目录（占位，由仓库作者补）；updater 更新前把旧版本备份到这里，回滚时还原
const BACKUP_DIR: &str = "<备份目录，仓库作者补>";

/// 便携版安装选项（任务 06 规格 3）
pub struct PortableOptions {
    /// 新版本号（暂存目录与 `update.source` 路径的一部分）
    pub version: String,
    /// 目标目录（便携版 = exe 所在目录，即 `app_paths::root_dir()`）
    pub target_dir: PathBuf,
    /// 更新时保留的路径（便携版 = `[data_dir()]`，用户数据不随更新清空）
    pub preserve_paths: Vec<PathBuf>,
    /// 原启动参数（透传给新进程，与 NSIS 的 `/ARGS` 语义一致）
    pub launch_args: Vec<OsString>,
    /// 新版本可执行文件名（如 `rollcaller.exe`）
    pub exe_name: String,
    /// spawn updater 成功后的退出前钩子（清理单实例锁/托盘/持久化状态）
    ///
    /// 与任务 05 的 `NsisOptions.on_before_exit` 同款设计：`FnOnce` 只执行一次，
    /// 因此 `install_portable` 按值接收 `PortableOptions`（规格 3 写的是 `&`，
    /// 与 `FnOnce` 矛盾，取舍与 nsis.rs 保持一致：保留 `FnOnce` 语义 + 按值传参）。
    pub on_before_exit: Option<Box<dyn FnOnce() + Send>>,
}

/// 安装入口：解压 → 写 config.json → 分离式 spawn updater.exe → 钩子 → `exit(0)`
///
/// `bytes` 应为已验证（sha256 + 签名）的便携版 zip 载荷，验签是 03 的职责，本函数不再校验。
/// spawn 失败返回 `Err`（进程不退出，让上层提示用户）；成功路径由 updater 接管，本进程
/// 正常流程不可到达 `exit(0)` 之后。
#[cfg(windows)]
pub fn install_portable(bytes: &[u8], mut opts: PortableOptions) -> anyhow::Result<()> {
    // 1) 解压到暂存目录 <staging>/<version>/
    let staging_dir = staging_dir_for(&opts.version);
    extract_zip_to(bytes, &staging_dir)?;

    // 2) 写 config.json
    let config = build_updater_config(&opts, &staging_dir);
    let config_path = staging_dir.join(format!("{}-updater-config.json", opts.version));
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

    // 3) 分离式 spawn updater.exe <config.json>；失败返回 Err（进程不退出）
    spawn_updater(&config_path)?;

    // 4) 退出前钩子（清理单实例锁/托盘/持久化状态）
    if let Some(hook) = opts.on_before_exit.take() {
        hook();
    }

    // 5) 让渡：当前进程退出，updater 执行 wait → update → launch → (rollback)
    std::process::exit(0);
}

/// 非 Windows 平台：便携版安装不可用（桩）
#[cfg(not(windows))]
pub fn install_portable(_bytes: &[u8], _opts: PortableOptions) -> anyhow::Result<()> {
    anyhow::bail!("便携版安装仅支持 Windows")
}

/// 暂存目录 = `<UPDATER_STAGING_DIR>/<version>`
#[cfg(windows)]
fn staging_dir_for(version: &str) -> PathBuf {
    PathBuf::from(UPDATER_STAGING_DIR).join(version)
}

/// 解压 zip 到指定目录（含路径穿越防护）
///
/// 逐条检查 `ZipFile::enclosed_name()`：返回 `None` 说明条目路径非法（含 `..` 逃逸、
/// 绝对路径、Windows 盘符等），直接拒绝解压，防止恶意载荷写出目标目录之外。
#[cfg(windows)]
fn extract_zip_to(bytes: &[u8], dest: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dest)?;

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let Some(out_path) = file.enclosed_name() else {
            anyhow::bail!("zip 条目包含非法路径（路径穿越，拒绝解压）");
        };
        let out_path = dest.join(out_path);

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out)?;
        }
    }
    Ok(())
}

/// 分离式 spawn updater.exe（`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`）
///
/// 分离式：updater 不随本进程退出而终止，独立完成更新流程。
/// 失败返回 `Err`（进程不退出，由上层提示用户）。
#[cfg(windows)]
fn spawn_updater(config_path: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    std::process::Command::new(UPDATER_EXE)
        .arg(config_path)
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("spawn updater.exe 失败: {e}"))
}

// ---------------------------------------------------------------------------
// config.json 结构：字段与 D:\Go\updater\internal\config\schema.go 一一对应
//
// 注意：Go 端实际解析的字段是 `wait.forceKill` / `wait.interval`（schema.go），
// 其 README 示例中的 `forceKillAfterTimeout` / `checkInterval` 已过时——Go 的
// json.Unmarshal 会静默忽略未知字段，若按 README 生成，forceKill 将退回默认
// false，超时后不会强制结束旧进程。故这里以 schema.go 为准。
// ---------------------------------------------------------------------------

/// 顶层配置（schema.go:13-18）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdaterConfig {
    version: i64,
    runtime: RuntimeConfig,
    wait: WaitConfig,
    update: UpdateConfig,
    launch: LaunchConfig,
    rollback: RollbackConfig,
}

/// `runtime`（schema.go:45-46）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeConfig {
    headless: bool,
    log_file: String,
}

/// `wait`（schema.go:51-54）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WaitConfig {
    pid: i64,
    timeout: i64,
    force_kill: bool,
    interval: i64,
}

/// `update`（schema.go:59-63）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateConfig {
    source: String,
    target: String,
    preserve: Vec<String>,
    clean_before_copy: bool,
    backup: BackupConfig,
}

/// `update.backup`（schema.go:68-70）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupConfig {
    enabled: bool,
    location: String,
    exclude: Vec<String>,
}

/// `launch`（schema.go:75-77）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchConfig {
    execution: ExecutionConfig,
    context: ContextConfig,
    lifecycle: LifecycleConfig,
}

/// `launch.execution`（schema.go:82-84）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecutionConfig {
    mode: String,
    path: String,
}

/// `launch.context`（schema.go:89-91）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextConfig {
    workspace: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
}

/// `launch.lifecycle`（schema.go:96-97）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleConfig {
    stay_alive: i64,
    capture_output: bool,
}

/// `rollback`（schema.go:102-104）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RollbackConfig {
    enabled: bool,
    fallback_executable: String,
    max_attempts: i64,
}

/// 生成 updater 的 config.json 内容（纯函数，便于单测）
///
/// 占位说明：
/// - `update.source` = `<暂存目录>/<version>`（UPDATER_STAGING_DIR 占位）；
/// - `update.backup.location` / `rollback.fallbackExecutable` 基于 BACKUP_DIR 占位；
/// - `runtime.logFile` 建议用 `app_paths::logs_dir()` 拼接（规格 7 留空项）；
/// - `launch.execution.path` 用 `target_dir.join(exe_name)` 输出**绝对路径**——
///   Go 端 Validate 强制要求该字段为绝对路径（loader.go），仅文件名会被校验拒绝。
#[cfg(windows)]
fn build_updater_config(opts: &PortableOptions, staging_dir: &Path) -> UpdaterConfig {
    let pid = std::process::id() as i64;
    let target = opts.target_dir.to_string_lossy().into_owned();
    let preserve: Vec<String> = opts
        .preserve_paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let args: Vec<String> = opts
        .launch_args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let exe_path = opts.target_dir.join(&opts.exe_name).to_string_lossy().into_owned();
    let fallback = PathBuf::from(BACKUP_DIR)
        .join(&opts.exe_name)
        .to_string_lossy()
        .into_owned();
    let log_file = crate::config::app_paths::logs_dir()
        .join("updater.log")
        .to_string_lossy()
        .into_owned();

    UpdaterConfig {
        version: 1,
        runtime: RuntimeConfig {
            headless: true,
            log_file,
        },
        wait: WaitConfig {
            pid,
            timeout: 10_000,
            force_kill: true,
            interval: 300,
        },
        update: UpdateConfig {
            source: staging_dir.to_string_lossy().into_owned(),
            target,
            preserve: preserve.clone(),
            clean_before_copy: true,
            backup: BackupConfig {
                enabled: true,
                location: BACKUP_DIR.to_string(),
                exclude: preserve.clone(),
            },
        },
        launch: LaunchConfig {
            execution: ExecutionConfig {
                mode: "direct".to_string(),
                path: exe_path,
            },
            context: ContextConfig {
                workspace: opts.target_dir.to_string_lossy().into_owned(),
                args,
                env: BTreeMap::new(),
            },
            lifecycle: LifecycleConfig {
                stay_alive: 0,
                capture_output: true,
            },
        },
        rollback: RollbackConfig {
            enabled: true,
            fallback_executable: fallback,
            max_attempts: 2,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造仅含一个文件的 zip（Stored 不压缩）
    fn make_test_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;

        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            for (name, content) in entries {
                zip.start_file(
                    *name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Stored),
                )
                .expect("写入 zip 条目失败");
                zip.write_all(content).expect("写入 zip 内容失败");
            }
            zip.finish().expect("结束 zip 写入失败");
        }
        buf.into_inner()
    }

    // ------------------------------------------------------------------
    // config.json 生成（规格 5：字段齐全、JSON 合法、pid=当前进程、args 透传）
    // ------------------------------------------------------------------

    #[test]
    fn config_json_is_valid_and_complete() {
        let opts = PortableOptions {
            version: "1.2.3".to_string(),
            target_dir: PathBuf::from("C:\\Apps\\rollcaller"),
            preserve_paths: vec![PathBuf::from("C:\\Apps\\rollcaller\\data")],
            launch_args: vec![
                OsString::from("--port"),
                OsString::from("8080"),
                OsString::from("--name=张三"),
                OsString::from("some space"),
            ],
            exe_name: "rollcaller.exe".to_string(),
            on_before_exit: None,
        };
        let staging = PathBuf::from("C:\\staging\\1.2.3");

        let config = build_updater_config(&opts, &staging);
        let json = serde_json::to_string(&config).expect("config.json 序列化失败");

        // JSON 合法且字段齐全
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("config.json 反序列化失败");
        assert_eq!(value["version"], 1);
        assert_eq!(value["runtime"]["headless"], true);
        assert_eq!(value["wait"]["pid"], std::process::id() as i64);
        assert_eq!(value["wait"]["timeout"], 10_000);
        assert_eq!(value["wait"]["forceKill"], true);
        assert_eq!(value["wait"]["interval"], 300);
        assert_eq!(value["update"]["source"], "C:\\staging\\1.2.3");
        assert_eq!(value["update"]["target"], "C:\\Apps\\rollcaller");
        assert_eq!(
            value["update"]["preserve"][0],
            "C:\\Apps\\rollcaller\\data"
        );
        assert_eq!(value["update"]["cleanBeforeCopy"], true);
        assert_eq!(value["update"]["backup"]["enabled"], true);
        assert_eq!(value["launch"]["execution"]["mode"], "direct");
        assert_eq!(
            value["launch"]["execution"]["path"],
            "C:\\Apps\\rollcaller\\rollcaller.exe"
        );
        assert_eq!(value["launch"]["context"]["workspace"], "C:\\Apps\\rollcaller");
        assert_eq!(value["launch"]["context"]["env"], serde_json::json!({}));
        assert_eq!(value["launch"]["lifecycle"]["stayAlive"], 0);
        assert_eq!(value["launch"]["lifecycle"]["captureOutput"], true);
        assert_eq!(value["rollback"]["enabled"], true);
        assert_eq!(value["rollback"]["maxAttempts"], 2);
    }

    /// 原启动参数逐项透传（含中文/空格，规格 5 要求）
    #[test]
    fn config_json_passthrough_args_with_chinese_and_spaces() {
        let opts = PortableOptions {
            version: "1.0.0".to_string(),
            target_dir: PathBuf::from("C:\\Apps\\rollcaller"),
            preserve_paths: vec![PathBuf::from("C:\\Apps\\rollcaller\\data")],
            launch_args: vec![
                OsString::from("--备注=假期 作业"),
                OsString::from("--path"),
                OsString::from("C:\\Program Files\\My App"),
            ],
            exe_name: "rollcaller.exe".to_string(),
            on_before_exit: None,
        };
        let staging = PathBuf::from("C:\\staging\\1.0.0");

        let config = build_updater_config(&opts, &staging);
        let json = serde_json::to_string(&config).expect("序列化失败");
        let value: serde_json::Value = serde_json::from_str(&json).expect("反序列化失败");

        assert_eq!(
            value["launch"]["context"]["args"],
            serde_json::json!([
                "--备注=假期 作业",
                "--path",
                "C:\\Program Files\\My App"
            ])
        );
    }

    // ------------------------------------------------------------------
    // 解压（规格 5：解压到指定目录、路径穿越防护）
    // ------------------------------------------------------------------

    #[test]
    fn extract_zip_writes_to_dest_dir() {
        let zip_bytes = make_test_zip(&[("app.exe", b"MZ\x90\x00\x00fake"), ("data/x.txt", b"hi")]);
        let dest = tempfile::tempdir().expect("创建临时目录失败");

        extract_zip_to(&zip_bytes, dest.path()).expect("解压应成功");

        assert_eq!(
            std::fs::read(dest.path().join("app.exe")).expect("读取 app.exe 失败"),
            b"MZ\x90\x00\x00fake"
        );
        assert_eq!(
            std::fs::read(dest.path().join("data/x.txt")).expect("读取 x.txt 失败"),
            b"hi"
        );
    }

    /// 路径穿越防护：`../evil.txt` 条目必须被拒绝，且目标目录外不产生任何文件
    #[test]
    fn extract_zip_rejects_path_traversal() {
        let zip_bytes = make_test_zip(&[("../evil.txt", b"evil")]);
        let dest = tempfile::tempdir().expect("创建临时目录失败");

        let result = extract_zip_to(&zip_bytes, dest.path());
        assert!(result.is_err(), "含 `..` 的 zip 应被拒绝");

        // 目标目录外不产生文件
        assert!(!dest.path().parent().unwrap().join("evil.txt").exists());
    }

    // ------------------------------------------------------------------
    // spawn 失败路径（规格 5：传不存在的 updater.exe → Err 且进程不退出）
    // ------------------------------------------------------------------

    #[test]
    fn spawn_updater_failure_returns_err() {
        // UPDATER_EXE 为占位常量（路径不存在），spawn 必然失败 → Err
        let result = spawn_updater(Path::new("unused-config.json"));
        assert!(result.is_err(), "spawn 不存在的 updater.exe 应返回 Err");
    }
}
