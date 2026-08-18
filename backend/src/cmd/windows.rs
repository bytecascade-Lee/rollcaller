use crate::windows::{app_window, help_window};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn windows_list(app: AppHandle) -> Vec<String> {
    app.webview_windows().keys().cloned().collect()
}

#[tauri::command]
pub async fn windows_app_open(app: AppHandle) -> Result<(), String> {
    app_window::open(app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn windows_help_open(app: AppHandle) -> Result<(), String> {
    help_window::open(app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn windows_help_hide(app: AppHandle) -> Result<(), String> {
    help_window::hide(app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn windows_help_close(app: AppHandle) -> Result<(), String> {
    help_window::close(app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn windows_help_destroy(app: AppHandle) -> Result<(), String> {
    help_window::destroy(app).map_err(|e| e.to_string())
}
