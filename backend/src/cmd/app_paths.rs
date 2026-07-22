use crate::config::app_paths;
use crate::config::app_paths::AppMode;

#[tauri::command]
pub fn data_dir() -> String {
    app_paths::data_dir().to_str().unwrap().to_string()
}

#[tauri::command]
pub fn config_dir() -> String {
    app_paths::config_dir().to_str().unwrap().to_string()
}

#[tauri::command]
pub fn cache_dir() -> String {
    app_paths::cache_dir().to_str().unwrap().to_string()
}

#[tauri::command]
pub fn logs_dir() -> String {
    app_paths::logs_dir().to_str().unwrap().to_string()
}

#[tauri::command]
pub fn temp_dir() -> String {
    app_paths::temp_dir().to_str().unwrap().to_string()
}

#[tauri::command]
pub fn webview_dir() -> String {
    app_paths::webview_dir().to_str().unwrap().to_string()
}

#[tauri::command]
pub fn resources_dir() -> String {
    app_paths::resources_dir().to_str().unwrap().to_string()
}

#[tauri::command]
pub fn app_mode() -> AppMode {
    app_paths::current_mode()
}

#[tauri::command]
pub fn is_customized_dir() -> bool {
    app_paths::is_customized_dir()
}
