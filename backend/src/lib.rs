use crate::config::{app_config, app_paths, logger};
use crate::windows::app_window;
use tauri::WebviewWindowBuilder;

mod bootstrap;
mod cmd;
mod common;
mod config;
mod database;
mod repo;
mod service;
mod util;
mod windows;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    tauri::Builder::default()
        .setup(|app| init(app))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_decorum::init())
        .plugin(tauri_plugin_tracing::Builder::new().build())
        .plugin(tauri_plugin_prevent_default::init_with_manual_injection())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cmd| {}))
        .invoke_handler(tauri::generate_handler![
            crate::cmd::attendance_status::attendance_status_list,
            crate::cmd::attendance_status::attendance_status_create,
            crate::cmd::attendance_status::attendance_status_update,
            crate::cmd::app_info::app_info,
            crate::cmd::app_paths::root_dir,
            crate::cmd::app_paths::config_dir,
            crate::cmd::app_paths::data_dir,
            crate::cmd::app_paths::cache_dir,
            crate::cmd::app_paths::logs_dir,
            crate::cmd::app_paths::temp_dir,
            crate::cmd::app_paths::webview2_dir,
            crate::cmd::app_paths::resources_dir,
            crate::cmd::app_paths::app_mode,
            crate::cmd::app_paths::is_customized_dir,
            crate::cmd::help::help_load_markdown,
            crate::cmd::help::help_load_readme,
            crate::cmd::help::help_load_license,
            crate::cmd::help::help_load_changelog,
            crate::cmd::help::help_load_release_notes,
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
            crate::cmd::export::record_export,
            crate::cmd::tts::tts_cloud_model,
            crate::cmd::windows::windows_app_open,
            crate::cmd::windows::windows_help_open,
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
    let handle2 = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = bootstrap::init().await {
            bootstrap::show_fatal_error(&handle, e);
        }
    });

    if let Err(e) = app_window::open(handle2) {
        bootstrap::show_fatal_error(app.handle(), e);
    }
    Ok(())
}
