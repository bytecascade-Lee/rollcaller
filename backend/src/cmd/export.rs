use crate::database::database_pool;
use crate::service::export_service;
use std::path::Path;

#[tauri::command]
pub async fn student_export(path: String, ids: Vec<i64>) -> Result<(), String> {
    let path = Path::new(&path);
    let rb = database_pool::database().await.map_err(|e| e.to_string())?;
    export_service::export_students(&*rb, path, ids).await.map_err(|e| e.to_string())?;
    Ok(())
}
