use crate::cmd::ipc_err;
use crate::service::tts_service;

#[tauri::command]
pub async fn tts_cloud_model(name: String) -> Result<String, String> {
    tts_service::generate_by_cloud_model(name)
        .await
        .map_err(|e| ipc_err("tts_cloud_model", e))
}
