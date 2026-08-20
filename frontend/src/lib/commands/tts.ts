import { invoke } from "@tauri-apps/api/core";

/** 云端 TTS：调用 API 生成音频并缓存 */
export async function speak(name: string, studentNo?: string) {
  return await invoke<void>("tts_speak", { name, studentNo: studentNo ?? null });
}

/** 云端 TTS：获取缓存音频的 Base64 数据 */
export async function getAudio(name: string, studentNo?: string) {
  return await invoke<string>("tts_get_audio", { name, studentNo: studentNo ?? null });
}

/** 云端 TTS：后端播放已缓存的音频 */
export async function play(name: string, studentNo?: string) {
  return await invoke<void>("tts_play", { name, studentNo: studentNo ?? null });
}
