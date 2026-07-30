use crate::config::{app_config, app_paths, logger};
use crate::database::database_bootstrap;
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
    #[cfg(debug_assertions)]
    println!("{:?}", app_config::AppConfig::get(""));

    logger::init();
    database_bootstrap::init().await.expect("Failed to run database migrations.");

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
            crate::cmd::record::record_list,
            crate::cmd::record::reocrd_batch_update_attendance_status,
            crate::cmd::record::reocrd_batch_update_remark,
            crate::cmd::rollcall::roll_call_pick,
            crate::cmd::import::preview_excel,
            crate::cmd::import::import_excel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    WebviewWindowBuilder::new(app, "main", Default::default())
        .data_directory(app_paths::webview_dir().to_path_buf())
        .inner_size(900.0, 700.0)
        .auto_resize()
        .build()
        .expect("error while creating window");
    Ok(())
}
