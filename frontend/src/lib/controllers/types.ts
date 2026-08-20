/** TTS 模式 */
export type TTSMode =
  | "system-local"  // 浏览器 Web Speech API
  | "ai-local"      // 本地 AI 模型（预留）
  | "ai-cloud";     // 云端 AI 大模型（MIMO API）

/** 队列项状态 */
export type TtsItemStatus = "loading" | "ready" | "playing" | "done";

/** 播报队列项 */
export type TtsQueueItem = {
  id: string;
  studentNo: string;
  name: string;
  status: TtsItemStatus;
  audioUrl?: string;
  error?: string;
};
