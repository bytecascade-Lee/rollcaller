import {invoke} from "@tauri-apps/api/core";

/** 云端 TTS：调用 API 生成音频并获取Base64编码值 */
export async function speak(studentNo: string, name: string) {
  return await invoke<string>("tts_cloud_model", {studentNo, name});
}
