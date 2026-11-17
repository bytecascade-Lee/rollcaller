use crate::cmd::ipc_err;
use crate::service::tts;

#[tauri::command]
pub async fn tts_cloud_model(name: String) -> Result<String, String> {
    tts::generate_by_cloud_model(name)
        .await
        .map_err(|e| ipc_err("tts_cloud_model", e))
}
