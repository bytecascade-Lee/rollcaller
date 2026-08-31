use crate::common::enums::tts::TtsMode;
use crate::common::ext::hash_ext::HashExt;
use crate::config::app_paths;
use crate::state::http_client;
use anyhow::Context;
use base64::Engine;
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use tracing::info;

const API_KEY: &str = env!("MIMO_TTS_API_KEY");
/// 调用云端 TTS API 并缓存音频
pub async fn generate_by_cloud_model(name: String) -> anyhow::Result<String> {
    if check_cache(&name)? {
        info!("Cache hit for TTS: {}", &name);
        return Ok(base64::engine::general_purpose::STANDARD.encode(get_audio_bytes(&name)?));
    }
    info!("Cache miss for TTS: {}, calling API", &name);

    let response = http_client::get_client()
        .post("https://api.xiaomimimo.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", API_KEY))
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
                    "content": &name,
                }
            ],
            "audio": {
                "format": "wav",
                "voice": "冰糖"
            }
        }))
        .send()
        .await
        .context("请求云端TTS API失败")?;

    let data = response.json::<serde_json::Value>().await.context("解析TTS API响应失败")?;
    let audio_b64 = data["choices"][0]["message"]["audio"]["data"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("TTS API返回数据中缺少音频字段"))?;

    let audio_bytes = base64::engine::general_purpose::STANDARD.decode(audio_b64)?;

    let cache_path = get_cache_path(&name);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(&cache_path)?;
    file.write_all(&audio_bytes)?;

    info!("TTS audio cached at {:?} for {}", cache_path, &name);
    Ok(audio_b64.to_string())
}

/// 获取缓存的音频字节数据
pub fn get_audio_bytes(name: &str) -> anyhow::Result<Vec<u8>> {
    let cache_path = get_cache_path(name);
    std::fs::read(&cache_path)
        .with_context(|| format!("读取音频缓存失败: {:?}", cache_path))
}

/// 检查缓存是否存在且有效
fn check_cache(name: &str) -> anyhow::Result<bool> {
    let cache_file = get_cache_path(name);

    if !cache_file.exists() {
        return Ok(false);
    }

    let metadata = cache_file.metadata()?;
    if metadata.len() == 0 {
        std::fs::remove_file(&cache_file)?;
        return Ok(false);
    }

    Ok(true)
}

/// 构造缓存文件路径：sha256(student_no + name)
fn get_cache_path(name: &str) -> std::path::PathBuf {
    let hash = name.sha256();

    app_paths::cache_dir()
        .join("tts/ai-cloud")
        .join(&hash[0..2])
        .join(format!("{}.wav", hash))
}
