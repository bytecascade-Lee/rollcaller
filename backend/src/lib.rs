use crate::config::{app_paths, logger};
use crate::database::database_bootstrap;
use tauri::WebviewWindowBuilder;

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
    database_bootstrap::init().await.expect("Failed to run database migrations.");

    tauri::Builder::default()
        .setup(|app| create_window(app))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn create_window(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    WebviewWindowBuilder::new(app, "main", Default::default())
        .data_directory(app_paths::webview_dir().to_path_buf())
        .inner_size(800.0, 600.0)
        .auto_resize()
        .build()
        .expect("error while creating window");
    Ok(())
}
