use crate::common::ext::hash_ext::HashExt;
use crate::config::app_paths;
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::Write;
use tracing::info;

pub async fn api(student_no: String, name: String) -> anyhow::Result<()> {
    // 检查缓存 - 修复：使用 ? 处理 Result
    if check_cache(&student_no, &name)? {
        info!("Cache hit for student: {}-{}", student_no, name);
        return Ok(());
    }
    info!("Cache miss for student: {}, calling API", student_no);

    let client = reqwest::Client::new();

    let response = client
        .post("https://api.xiaomimimo.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", env::var("MIMO_TTS_API_KEY")?))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "mimo-v2.5-tts",
            "messages": [
                {
                    "role": "user",
                    "content": "Speak in a clear, teacher-like tone. Project your voice as if in a classroom. Normal pace, with natural rising and falling pitch. Sound warm and encouraging, but not overly excited."
                },
                {
                    "role": "assistant",
                    "content": name,
                }
            ],
            "audio": {
                "format": "wav",
                "voice": "冰糖"
            }
        }))
        .send()
        .await?;

    let data: serde_json::Value = response.json().await?;
    let audio_b64 = data["choices"][0]["message"]["audio"]["data"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing audio data"))?;  // 修复：使用 ok_or_else 避免 unwrap

    let audio_bytes = base64::decode(audio_b64)?;

    // 修复：确保目录存在
    let cache_path = get_cache_path(&student_no, &name);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(cache_path)?;
    file.write_all(&audio_bytes)?;

    Ok(())
}

/// 检查缓存是否存在且有效
///
/// # Arguments
/// * `student_no` - 学号
/// * `name` - 姓名
///
/// # Returns
/// * `Result<bool>` - Ok(true)表示缓存命中，Ok(false)表示未命中，Err表示发生错误
fn check_cache(student_no: &str, name: &str) -> anyhow::Result<bool> {
    // 1. 获取缓存路径 - 修复：去掉多余的 &
    let cache_file = get_cache_path(student_no, name);

    // 2. 检查文件是否存在
    if !cache_file.exists() {
        return Ok(false);
    }

    // 3. 验证文件大小
    let metadata = cache_file.metadata()?;
    if metadata.len() == 0 {
        // 文件为空，视为无效缓存，可以删除它
        std::fs::remove_file(&cache_file)?;  // 修复：需要引用
        return Ok(false);
    }

    Ok(true)
}

/// 构造缓存文件路径
///
/// # Arguments
/// * `student_no` - 学号
/// * `name` - 姓名
///
/// # Returns
/// * `PathBuf` - 完整的缓存文件路径
fn get_cache_path(student_no: &str, name: &str) -> std::path::PathBuf {
    let combined = format!("{}{}", student_no, name);
    let hash = combined.sha256();

    let folder_name = &hash[0..2];
    let file_name = format!("{}.wav", hash);

    app_paths::cache_dir()
        .join("tts/api")
        .join(folder_name)
        .join(file_name)
}
