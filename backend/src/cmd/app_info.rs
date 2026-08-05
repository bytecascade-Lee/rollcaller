use crate::common::entity::app_info::AppInfo;

#[tauri::command]
pub fn app_info() -> AppInfo {
    let app_info = AppInfo::new();
    Debug!("{:#?}", app_info);
    app_info
}
