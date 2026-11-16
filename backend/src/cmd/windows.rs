use crate::cmd::ipc_err;
use crate::windows::{app_window, help_window};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn windows_list(app: AppHandle) -> Vec<String> {
    app.webview_windows().keys().cloned().collect()
}

#[tauri::command]
pub async fn windows_app_open(app: AppHandle) -> Result<(), String> {
    app_window::open(&app).map_err(|e| ipc_err("windows_app_open", e))
}

#[tauri::command]
pub async fn windows_help_open(app: AppHandle) -> Result<(), String> {
    help_window::open(&app).map_err(|e| ipc_err("windows_help_open", e))
}
