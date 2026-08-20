use crate::common::ext::hash_ext::HashExt;
use crate::config::app_paths;
use anyhow::Context;
use base64::Engine;
use rodio::{Decoder, DeviceSinkBuilder, Player};
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use tracing::info;

/// 调用云端 TTS API 并缓存音频
///
/// # Arguments
/// * `name` - 要合成的文本
/// * `student_no` - 学号（可选，用于缓存键）
pub async fn api(name: String, student_no: Option<String>) -> anyhow::Result<()> {
    let cache_key = student_no.unwrap_or_else(|| uuid_v4());
    if check_cache(&cache_key, &name)? {
        info!("Cache hit for TTS: {}", name);
        return Ok(());
    }
    info!("Cache miss for TTS: {}, calling API", name);

    let api_key = env::var("MIMO_TTS_API_KEY")
        .context("环境变量 MIMO_TTS_API_KEY 未设置，请在 .env 文件中配置云端TTS密钥")?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.xiaomimimo.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
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
        .await
        .context("请求云端TTS API失败")?;

    let data: serde_json::Value = response.json().await.context("解析TTS API响应失败")?;
    let audio_b64 = data["choices"][0]["message"]["audio"]["data"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("TTS API返回数据中缺少音频字段"))?;

    let audio_bytes = base64::engine::general_purpose::STANDARD.decode(audio_b64)?;

    let cache_path = get_cache_path(&cache_key, &name);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(&cache_path)?;
    file.write_all(&audio_bytes)?;

    info!("TTS audio cached: {:?}", cache_path);
    Ok(())
}

/// 播放缓存中的音频文件
///
/// 使用 rodio 0.22 API：DeviceSinkBuilder + Player
///
/// # Arguments
/// * `name` - 要合成的文本
/// * `student_no` - 学号（可选，用于缓存键）
pub fn play(name: String, student_no: Option<String>) -> anyhow::Result<()> {
    let cache_key = student_no.unwrap_or_else(|| uuid_v4());
    let cache_path = get_cache_path(&cache_key, &name);

    if !cache_path.exists() {
        return Err(anyhow::anyhow!(
            "音频缓存不存在，请先调用 tts_speak 获取音频"
        ));
    }

    let file = File::open(&cache_path)
        .with_context(|| format!("打开音频缓存文件失败: {:?}", cache_path))?;
    let buf_reader = BufReader::new(file);

    // rodio 0.22: DeviceSinkBuilder 创建音频设备，Player 创建播放控制器
    let sink_handle = DeviceSinkBuilder::open_default_sink()
        .context("初始化音频输出设备失败，请检查系统音频设备")?;
    let mixer = sink_handle.mixer();
    let player = Player::connect_new(mixer);

    let source = Decoder::new(buf_reader).context("解码WAV音频失败，文件可能已损坏")?;
    player.append(source);

    // 阻塞等待播放完成（保持 sink_handle 存活，否则音频输出会中断）
    std::thread::sleep(std::time::Duration::from_secs(5));
    Ok(())
}

/// 获取缓存的音频字节数据
pub fn get_audio_bytes(name: &str, student_no: Option<&str>) -> anyhow::Result<Vec<u8>> {
    let cache_key = student_no.unwrap_or("");
    let cache_path = get_cache_path(cache_key, name);

    std::fs::read(&cache_path)
        .with_context(|| format!("读取音频缓存失败: {:?}", cache_path))
}

fn check_cache(cache_key: &str, name: &str) -> anyhow::Result<bool> {
    let cache_file = get_cache_path(cache_key, name);

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

fn get_cache_path(cache_key: &str, name: &str) -> std::path::PathBuf {
    let combined = format!("{}{}", cache_key, name);
    let hash = combined.sha256();

    let folder_name = &hash[0..2];
    let file_name = format!("{}.wav", hash);

    app_paths::cache_dir()
        .join("tts/api")
        .join(folder_name)
        .join(file_name)
}

/// 生成简易 UUID v4
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:032x}", t)
}
