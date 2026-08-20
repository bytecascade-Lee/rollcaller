import { TTSCommand } from "$commands";
import { tts } from "./WebViewTtsService.svelte";

/** TTS 模式：local = 浏览器本地合成，cloud = 云端大模型 */
export type TTSMode = "local" | "cloud";

/** 全局 TTS 模式状态，供组件绑定 */
export const ttsMode = $state<{ value: TTSMode }>({ value: "local" });

/**
 * TTS 协调控制器
 *
 * 职责：
 * - 对外统一接口：speakNow(studentId, name)
 * - 根据当前模式路由到本地 WebView TTS 或云端 API
 * - 本地模式只用 name；云端模式传 studentId + name 给后端（缓存键需要双字段）
 */
class TtsController {
  /** 当前 TTS 模式 */
  get mode(): TTSMode {
    return ttsMode.value;
  }
  set mode(v: TTSMode) {
    ttsMode.value = v;
  }

  /** 是否正在播报 */
  get speaking() {
    return tts.speaking;
  }

  /** 是否暂停 */
  get paused() {
    return tts.paused;
  }

  /** 当前播报文本 */
  get currentText() {
    return tts.currentText;
  }

  /**
   * 立即播报（打断当前，清空队列）
   *
   * @param studentId - 学号（云端模式用于缓存键）
   * @param name      - 学生姓名（要播报的文本）
   */
  async speakNow(studentId: bigint, name: string) {
    if (ttsMode.value === "cloud") {
      await this.#speakCloud(studentId, name);
    } else {
      tts.speakNow(name);
    }
  }

  /**
   * 追加到队列
   */
  speak(studentId: bigint, name: string) {
    if (ttsMode.value === "cloud") {
      // 云端模式异步，fire-and-forget
      void this.#speakCloud(studentId, name);
    } else {
      tts.speak(name);
    }
  }

  /** 暂停 */
  pause() {
    tts.pause();
  }

  /** 恢复 */
  resume() {
    tts.resume();
  }

  /** 停止并清空队列 */
  cancel() {
    tts.cancel();
  }

  /** 清空队列（不影响当前播报） */
  clearQueue() {
    tts.clearQueue();
  }

  // ─── 云端模式实现 ────────────────────────────────────

  async #speakCloud(studentId: bigint, name: string) {
    try {
      const studentNo = String(studentId);

      // 1. 后端：检查缓存 / 调用 API / 写入缓存
      await TTSCommand.speak(name, studentNo);

      // 2. 获取 Base64 音频数据
      const b64 = await TTSCommand.getAudio(name, studentNo);

      // 3. 解码并播放
      const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
      const blob = new Blob([bytes], { type: "audio/wav" });
      const url = URL.createObjectURL(blob);

      await new Promise<void>((resolve, reject) => {
        const audio = new Audio(url);
        audio.onended = () => {
          URL.revokeObjectURL(url);
          resolve();
        };
        audio.onerror = () => {
          URL.revokeObjectURL(url);
          reject(new Error("音频播放失败"));
        };
        audio.play().catch(reject);
      });
    } catch (e) {
      console.error("Cloud TTS Error:", e);
    }
  }
}

export const ttsController = new TtsController();
