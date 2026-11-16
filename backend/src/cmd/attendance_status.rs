use crate::cmd::ipc_err;
use crate::common::entity::attendance_status::AttendanceStatus;
use crate::database::database_pool;
use crate::service::attendance_status_service;

#[tauri::command]
pub async fn attendance_status_list() -> Result<Vec<AttendanceStatus>, String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| ipc_err("attendance_status_list", e))?;
    attendance_status_service::get_all(&rb)
        .await
        .map_err(|e| ipc_err("attendance_status_list", e))
}

#[tauri::command]
pub async fn attendance_status_create(attendance_status: AttendanceStatus) -> Result<(), String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| ipc_err("attendance_status_create", e))?;
    attendance_status_service::create(&rb, attendance_status)
        .await
        .map_err(|e| ipc_err("attendance_status_create", e))
}

#[tauri::command]
pub async fn attendance_status_update(attendance_status: AttendanceStatus) -> Result<(), String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| ipc_err("attendance_status_update", e))?;
    attendance_status_service::update(&rb, attendance_status)
        .await
        .map_err(|e| ipc_err("attendance_status_update", e))
}
