use base64::Engine;
use crate::service::tts_service;

/// 云端 TTS：调用 API 生成音频并缓存
///
/// 返回 Ok(()) 表示音频已缓存就绪
#[tauri::command]
pub async fn tts_speak(name: String, student_no: Option<String>) -> Result<(), String> {
    tts_service::api(name, student_no)
        .await
        .map_err(|e| e.to_string())
}

/// 云端 TTS：播放已缓存的音频（阻塞直到播放完成）
#[tauri::command]
pub fn tts_play(name: String, student_no: Option<String>) -> Result<(), String> {
    tts_service::play(name, student_no).map_err(|e| e.to_string())
}

/// 获取缓存的音频字节数据（Base64 编码），供前端播放
#[tauri::command]
pub fn tts_get_audio(name: String, student_no: Option<String>) -> Result<String, String> {
    let bytes =
        tts_service::get_audio_bytes(&name, student_no.as_deref()).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}
