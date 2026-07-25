use crate::common::entity::record::RollcallRecord;
use crate::database::database_pool;
use crate::service::rollcall_service;

/// 点名：从学生列表中随机选一人，写入记录，返回完整点名记录
#[tauri::command]
pub async fn roll_call_pick(
    student_ids: Vec<i64>,
    session_id: String,
) -> Result<RollcallRecord, String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    rollcall_service::pick_and_save(&*rb, student_ids, session_id)
        .await
        .map_err(|e| e.to_string())
}
