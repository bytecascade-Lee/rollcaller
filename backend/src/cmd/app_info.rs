use crate::config::app_info;

#[tauri::command]
pub fn app_info() -> app_info::AppInfo {
    *app_info::app_info()
}
