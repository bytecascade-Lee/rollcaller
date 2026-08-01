use crate::config::{app_config, app_paths, logger};
use tauri::WebviewWindowBuilder;

mod bootstrap;
mod cmd;
mod common;
mod config;
mod database;
mod repo;
mod service;
mod util;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    logger::init();
    tauri::Builder::default()
        .setup(|app| init(app))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            crate::cmd::app_paths::data_dir,
            crate::cmd::app_paths::config_dir,
            crate::cmd::app_paths::cache_dir,
            crate::cmd::app_paths::logs_dir,
            crate::cmd::app_paths::temp_dir,
            crate::cmd::app_paths::webview_dir,
            crate::cmd::app_paths::resources_dir,
            crate::cmd::app_paths::app_mode,
            crate::cmd::app_paths::is_customized_dir,
            crate::cmd::student::student_list,
            crate::cmd::student::student_single_create,
            crate::cmd::student::student_single_update,
            crate::cmd::student::student_batch_delete,
            crate::cmd::student::student_batch_restore,
            crate::cmd::record::record_list,
            crate::cmd::record::record_single_create,
            crate::cmd::record::record_batch_update,
            crate::cmd::record::record_batch_update_attendance_status,
            crate::cmd::record::record_batch_update_remark,
            crate::cmd::rollcall::pick,
            crate::cmd::import::preview_excel,
            crate::cmd::import::import_excel,
            crate::cmd::export::student_export,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Tauri setup 钩子：创建主窗口并异步执行启动初始化
///
/// 初始化（目录准备、数据目录读写校验、数据库迁移）通过
/// `tauri::async_runtime::spawn` 异步执行，避免阻塞主线程；
/// 失败时弹出原生错误对话框提示，解决打包后启动失败无任何报错输出的问题。
fn init(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = bootstrap::init().await {
            show_fatal_error(&handle, e);
        }
    });

    if let Err(e) = WebviewWindowBuilder::new(app, "main", Default::default())
        .data_directory(app_paths::webview_dir().to_path_buf())
        .inner_size(900.0, 700.0)
        .auto_resize()
        .build()
    {
        show_fatal_error(app.handle(), anyhow::Error::msg(e.to_string()));
    }
    Ok(())
}

/// 弹出原生错误对话框并退出应用
///
/// `blocking_show()` 会阻塞当前线程直到用户点击对话框，不能直接在 Tauri
/// 主线程中调用（否则会冻结应用），因此在独立线程中执行；用户确认后退出应用。
///
/// 初始化阶段 tracing 日志模块可能不可用（日志目录/订阅器初始化失败），
/// 因此除 tracing 外，同时向控制台输出，并直接写一份 fatal-error.log 兜底。
fn show_fatal_error(handle: &tauri::AppHandle, error: anyhow::Error) {
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
fn write_fatal_log(message: &str) -> std::io::Result<()> {
    let logs_dir = app_paths::logs_dir();
    std::fs::create_dir_all(logs_dir)?;
    std::fs::write(logs_dir.join("fatal-error.log"), message)
}
