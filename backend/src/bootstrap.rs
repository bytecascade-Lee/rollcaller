use crate::config::{app_paths, logger};
use crate::database::{database_bootstrap, database_pool};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io;
use std::path::Path;
use tracing::info;

/// 启动初始化：统筹整个启动流程
///
/// 执行顺序：
/// 1. 创建应用运行所需的全部目录（config / data / cache / temp / logs / webview）
/// 2. 校验数据目录可读可写（打包后的常见失败点：安装目录不可写）
/// 3. 初始化数据库连接池并执行迁移
///
/// 任一步骤失败都会返回错误，由调用方（Tauri setup 钩子）弹出原生对话框提示，
/// 避免打包后应用启动失败却无任何报错输出。
pub async fn init() -> Result<()> {
    println!(
        "root dir = {:?}\nconfig dir = {:?}\ndata dir = {:?}\ncache dir = {:?}\nlogs dir = {:?}\ntemp dir = {:?}\nresources dir = {:?}\nwebview2 dir = {:?}",
        app_paths::root_dir(),
        app_paths::config_dir(),
        app_paths::data_dir(),
        app_paths::cache_dir(),
        app_paths::logs_dir(),
        app_paths::temp_dir(),
        app_paths::resources_dir(),
        app_paths::webview2_dir()
    );
    ensure_directories()?;
    ensure_data_dir_writable()?;
    logger::init();
    database_bootstrap::init().await?;
    register_exit_hooks();
    Ok(())
}

/// 注册进程退出前的清理钩子（供更新安装链路 spawn 成功后统一执行）
///
/// 当前仅数据库连接池需要显式关闭；单实例锁等资源随进程退出由 OS 自动释放，
/// 无需注册。
fn register_exit_hooks() {
    crate::shutdown_hooks::register(|| async {
        if let Err(e) = database_pool::close().await {
            tracing::error!("退出前关闭数据库失败：{e}");
        }
    });
}

/// 创建应用运行所需的全部目录
fn ensure_directories() -> Result<()> {
    let dirs = [
        app_paths::config_dir(),
        app_paths::data_dir(),
        app_paths::cache_dir(),
        app_paths::temp_dir(),
        app_paths::logs_dir(),
        app_paths::webview2_dir(),
    ];
    for dir in dirs {
        fs::create_dir_all(dir).with_context(|| format!("创建目录失败: {}", dir.display()))?;
        info!("目录已就绪: {}", dir.display());
    }
    Ok(())
}

/// 校验数据目录可读可写
///
/// 通过探针文件验证实际读写能力。仅检查路径存在性不足以发现权限问题，
/// 例如安装模式下应用被装到 Program Files 等不可写目录。
fn ensure_data_dir_writable() -> Result<()> {
    let probe = app_paths::data_dir().join(".write_probe");
    ensure_read_write(&probe).map_err(|e| anyhow!("数据目录不可读或不可写: {} ({})", app_paths::data_dir().display(), e))?;
    let _ = fs::remove_file(&probe);
    Ok(())
}

/// 检查路径是否存在、可读、可写，不存在则创建
///
/// # 参数
/// - `path`: 要检查的路径
///
/// # 返回
/// - `Ok(())`: 路径存在且可读写
/// - `Err(io::Error)`: 出错时返回错误信息
pub fn ensure_read_write(path: &Path) -> io::Result<()> {
    // 1. 检查是否存在，不存在则创建（包括父目录）
    if !path.exists() {
        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // 创建文件
        fs::File::create(path)?;
    }

    // 2. 检查是否可读
    // 尝试打开文件读取
    match fs::File::open(path) {
        Ok(_) => (),
        Err(e) => return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("文件不可读: {:?}, 错误: {}", path, e),
        )),
    }

    // 3. 检查是否可写
    // 尝试以读写模式打开文件
    match fs::OpenOptions::new().write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("文件不可写: {:?}, 错误: {}", path, e),
        )),
    }
}

/// 弹出原生错误对话框并退出应用
///
/// `blocking_show()` 会阻塞当前线程直到用户点击对话框，不能直接在 Tauri
/// 主线程中调用（否则会冻结应用），因此在独立线程中执行；用户确认后退出应用。
///
/// 初始化阶段 tracing 日志模块可能不可用（日志目录/订阅器初始化失败），
/// 因此除 tracing 外，同时向控制台输出，并直接写一份 fatal-error.log 兜底。
pub fn show_fatal_error(handle: &tauri::AppHandle, error: anyhow::Error) {
    // 携带上下文的完整错误链（anyhow 的 {:#} 逐层展开 Caused by）
    let message = format!(
        "应用初始化失败，无法继续运行：\n\n{error:#}\n\n日志目录：{}\n本次错误另存于: fatal-error.log",
        app_paths::logs_dir().display()
    );

    tracing::error!("应用初始化失败: {:#}", error);
    // 控制台输出兜底（release 无控制台时被丢弃，不影响其他渠道）
    eprintln!("[fatal] {message}");
    // 直接写日志文件兜底：不依赖 tracing 订阅器是否成功初始化
    if let Err(e) = write_fatal_log(&message) {
        eprintln!("[fatal] 写入 fatal-error.log 失败: {e}");
    }

    let handle = handle.clone();
    std::thread::spawn(move || {
        use tauri_plugin_dialog::DialogExt;
        handle.dialog().message(message).title("启动失败").blocking_show();
        handle.exit(1);
    });
}

/// 将致命错误直接写入日志目录，绕过 tracing（初始化阶段其可能不可用）
fn write_fatal_log(message: &str) -> io::Result<()> {
    let logs_dir = app_paths::logs_dir();
    fs::create_dir_all(logs_dir)?;
    fs::write(logs_dir.join("setup-fatal-error.log"), message)
}
