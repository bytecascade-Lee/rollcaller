use base64::Engine;
use crate::service::tts_service;

/// 云端 TTS：调用 API 生成音频并缓存
///
/// # Arguments
/// * `student_no` - 学号（必传）
/// * `name` - 学生姓名
#[tauri::command]
pub async fn tts_speak(student_no: String, name: String) -> Result<(), String> {
    tts_service::api(&student_no, &name)
        .await
        .map_err(|e| e.to_string())
}

/// 获取缓存的音频字节数据（Base64 编码），供前端播放
///
/// # Arguments
/// * `student_no` - 学号（必传）
/// * `name` - 学生姓名
#[tauri::command]
pub fn tts_get_audio(student_no: String, name: String) -> Result<String, String> {
    let bytes =
        tts_service::get_audio_bytes(&student_no, &name).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}
