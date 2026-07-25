use crate::common::entity::student::{Student, StudentTable};
use crate::common::enums::student::{StudentSingleCreateResult, StudentSingleUpdate};
use crate::database::database_pool;
use crate::service::student_service;

/// 列出所有活跃学生
#[tauri::command]
pub async fn list_all_students() -> Result<Vec<StudentTable>, String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    student_service::get_all(&*rb).await.map_err(|e| e.to_string())
}

/// 创建单个学生
///
/// 返回 `StudentSingleCreateResult`，前端根据 `type` 字段处理不同状态：
/// - `Insert` / `Restore` / `Override` / `Retain` → 写入成功
/// - `ActiveExists` → 前端展示学号被占用的学生信息
/// - `Conflict` → 前端展示已删除记录与新记录的差异
#[tauri::command]
pub async fn create_student(student_no: String, name: String, overwrite: Option<bool>) -> Result<StudentSingleCreateResult, String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    student_service::create(&*rb, student_no, name, overwrite)
        .await
        .map_err(|e| e.to_string())
}

/// 更新学生信息
///
/// 若新学号与另一活跃学生冲突，返回错误。
#[tauri::command]
pub async fn update_student(student: Student) -> Result<StudentTable, String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    match student_service::update(&*rb, student).await {
        Ok(StudentSingleUpdate::Update(s)) => Ok(s),
        Ok(StudentSingleUpdate::Conflict(s)) => Err(format!("学号「{}」已被使用", s.student_no)),
        Err(e) => Err(e.to_string()),
    }
}

/// 删除学生（软删除）
#[tauri::command]
pub async fn delete_students(ids: Vec<i64>) -> Result<(), String> {
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    student_service::delete(&*rb, ids).await.map_err(|e| e.to_string())
}
