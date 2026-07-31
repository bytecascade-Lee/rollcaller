use crate::service::rollcall_service;

/// 点名：从学生列表中随机选一人，写入记录，返回完整点名记录
#[tauri::command]
pub fn pick(student_ids: Vec<i64>) -> Result<i64, String> {
    rollcall_service::pick(student_ids).map_err(|e| e.to_string())
}
