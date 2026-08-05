use crate::common::entity::app_info::AppInfo;
use tracing::debug;

#[tauri::command]
pub fn app_info() -> AppInfo {
    let app_info = AppInfo::new();
    debug!("{:#?}", app_info);
    app_info
}
