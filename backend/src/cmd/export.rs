use crate::cmd::ipc_err;
use crate::database::database_pool;
use crate::service::export;
use std::path::Path;

#[tauri::command]
pub async fn student_export(path: String, ids: Vec<i64>) -> Result<(), String> {
    let path = Path::new(&path);
    let rb = database_pool::database().await.map_err(|e| ipc_err("student_export", e))?;
    export::export_students(&rb, path, ids)
        .await
        .map_err(|e| ipc_err("student_export", e))?;
    Ok(())
}

#[tauri::command]
pub async fn record_export(path: String, ids: Vec<i64>) -> Result<(), String> {
    let path = Path::new(&path);
    let rb = database_pool::database().await.map_err(|e| ipc_err("record_export", e))?;
    export::export_records(&rb, path, ids)
        .await
        .map_err(|e| ipc_err("record_export", e))?;
    Ok(())
}
