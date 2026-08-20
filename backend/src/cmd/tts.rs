use crate::service::tts_service;

#[tauri::command]
pub async fn tts_cloud_model(student_no: String, name: String) -> Result<String, String> {
    tts_service::generate_by_cloud_model(&student_no, &name).await.map_err(|e| e.to_string())
}
