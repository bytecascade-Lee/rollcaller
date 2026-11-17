use crate::cmd::ipc_err;
use crate::common::entity::record::{Record, RollcallRecord};
use crate::database::database_pool;
use crate::service::record;

/// 列出最近的点名记录
#[tauri::command]
pub async fn record_list() -> Result<Vec<RollcallRecord>, String> {
    let rb = database_pool::database().await.map_err(|e| ipc_err("record_list", e))?;
    record::list_all(&rb).await.map_err(|e| ipc_err("record_list", e))
}

#[tauri::command]
pub async fn record_single_create(record: Record) -> Result<RollcallRecord, String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| ipc_err("record_single_create", e))?;
    record::create(&rb, record)
        .await
        .map_err(|e| ipc_err("record_single_create", e))
}

#[tauri::command]
pub async fn record_batch_update(
    ids: Vec<i64>,
    attendance_status: i8,
    remark: String,
) -> Result<Vec<RollcallRecord>, String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| ipc_err("record_batch_update", e))?;
    record::update(&rb, ids, attendance_status, remark)
        .await
        .map_err(|e| ipc_err("record_batch_update", e))
}

#[tauri::command]
pub async fn record_batch_update_attendance_status(
    ids: Vec<i64>,
    attendance_status: i8,
) -> Result<Vec<RollcallRecord>, String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| ipc_err("record_batch_update_attendance_status", e))?;
    record::update_attendance_status(&rb, ids, attendance_status)
        .await
        .map_err(|e| ipc_err("record_batch_update_attendance_status", e))
}

#[tauri::command]
pub async fn record_batch_update_remark(ids: Vec<i64>, remark: String) -> Result<Vec<RollcallRecord>, String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| ipc_err("record_batch_update_remark", e))?;
    record::update_remark(&rb, ids, remark)
        .await
        .map_err(|e| ipc_err("record_batch_update_remark", e))
}
