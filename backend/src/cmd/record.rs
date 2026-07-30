use crate::common::entity::record::RollcallRecord;
use crate::database::database_pool;
use crate::service::record_service;

/// 列出最近的点名记录
#[tauri::command]
pub async fn record_list() -> Result<Vec<RollcallRecord>, String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    record_service::list_all(&*rb).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn record_batch_update_attendance_status(ids: Vec<i64>, attendance_status: i8) -> Result<Vec<RollcallRecord>, String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    record_service::update_attendance_status(&*rb, ids, attendance_status)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn record_batch_update_remark(ids: Vec<i64>, remark: String) -> Result<Vec<RollcallRecord>, String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    record_service::update_remark(&*rb, ids, remark)
        .await
        .map_err(|e| e.to_string())
}
