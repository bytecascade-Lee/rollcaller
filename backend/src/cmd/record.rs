use crate::common::entity::record::RollcallRecord;
use crate::database::database_pool;
use crate::service::record_service;

/// 列出最近的点名记录
#[tauri::command]
pub async fn record_list() -> Result<Vec<RollcallRecord>, String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    record_service::list_all(&*rb).await.map_err(|e| e.to_string())
}
