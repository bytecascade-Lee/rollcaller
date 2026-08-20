import { TTSCommand } from "$commands";
import { tts } from "$services/WebViewTtsService.svelte.js";

/** TTS 模式：local = 浏览器本地合成，cloud = 云端大模型 */
export type TTSMode = "local" | "cloud";

/** 全局 TTS 模式状态，供组件绑定 */
export const ttsMode = $state<{ value: TTSMode }>({ value: "local" });

/**
 * TTS 协调控制器 —— 唯一队列权威
 *
 * 职责：
 * - 唯一队列，本地/云端统一调度
 * - speakNow(studentNo, name) 打断当前，清空队列，立即播报
 * - speak(studentNo, name) 追加到队列
 * - 云端串行：await 后端 → await Audio onended → 取下一个
 * - 本地：await tts.play(name) → onend resolve → 取下一个
 */
class TtsController {
  /** 统一播报队列 */
  #queue: Array<{ studentNo: string; name: string }> = [];
  /** 是否正在处理队列 */
  #processing = $state(false);
  /** 当前正在处理的条目 */
  #currentItem: { studentNo: string; name: string } | null = null;
  /** 云端模式：当前 Audio 元素，用于打断 */
  #cloudAudio: HTMLAudioElement | null = null;
  /** 打断标记：当前项的 await 链检测到此标记后跳出 */
  #skipCurrent = false;

  get mode(): TTSMode {
    return ttsMode.value;
  }
  set mode(v: TTSMode) {
    ttsMode.value = v;
  }

  get speaking() {
    return this.#processing;
  }

  get currentText() {
    return this.#currentItem?.name ?? null;
  }

  /**
   * 立即播报（打断当前，清空队列，立即播报新内容）
   */
  speakNow(studentNo: string, name: string) {
    // 清空队列
    this.#queue = [];
    // 打断当前
    this.#skipCurrent = true;
    if (this.#cloudAudio) {
      this.#cloudAudio.pause();
      this.#cloudAudio.src = "";
      this.#cloudAudio = null;
    }
    tts.cancel();
    // 重置状态
    this.#processing = false;
    this.#currentItem = null;
    this.#skipCurrent = false;
    // 立即播报
    this.#processItem(studentNo, name);
  }

  /**
   * 追加到队列
   */
  speak(studentNo: string, name: string) {
    this.#queue.push({ studentNo, name });
    if (!this.#processing) {
      this.#processNext();
    }
  }

  /** 暂停 */
  pause() {
    if (this.#cloudAudio) {
      this.#cloudAudio.pause();
    } else {
      tts.pause();
    }
  }

  /** 恢复 */
  resume() {
    if (this.#cloudAudio) {
      this.#cloudAudio.play();
    } else {
      tts.resume();
    }
  }

  /** 停止并清空队列 */
  cancel() {
    this.#skipCurrent = true;
    if (this.#cloudAudio) {
      this.#cloudAudio.pause();
      this.#cloudAudio.src = "";
      this.#cloudAudio = null;
    }
    tts.cancel();
    this.#queue = [];
    this.#processing = false;
    this.#currentItem = null;
    this.#skipCurrent = false;
  }

  /** 清空队列（不影响当前播报） */
  clearQueue() {
    this.#queue = [];
  }

  // ─── 内部：队列消费 ────────────────────────────────

  #processNext() {
    if (this.#queue.length === 0) {
      this.#processing = false;
      this.#currentItem = null;
      return;
    }

    const item = this.#queue.shift()!;
    this.#processItem(item.studentNo, item.name);
  }

  async #processItem(studentNo: string, name: string) {
    this.#processing = true;
    this.#currentItem = { studentNo, name };
    this.#skipCurrent = false;

    try {
      if (ttsMode.value === "cloud") {
        await this.#speakCloud(studentNo, name);
      } else {
        await this.#speakLocal(name);
      }
    } catch (e) {
      console.error("TTS Error:", e);
    }

    // 被 speakNow/cancel 打断时，不继续消费队列
    if (this.#skipCurrent) return;

    this.#processNext();
  }

  // ─── 本地模式 ──────────────────────────────────────

  async #speakLocal(name: string) {
    tts.cancel();
    await tts.play(name);
  }

  // ─── 云端模式：后端 API + 浏览器 Audio 播放 ────────

  async #speakCloud(studentNo: string, name: string) {
    // 1. 后端：检查缓存 / 调用 API / 写入缓存
    await TTSCommand.speak(studentNo, name);
    if (this.#skipCurrent) return;

    // 2. 获取 Base64 音频数据
    const b64 = await TTSCommand.getAudio(studentNo, name);
    if (this.#skipCurrent) return;

    // 3. 解码并播放，等待播放完成
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const blob = new Blob([bytes], { type: "audio/wav" });
    const url = URL.createObjectURL(blob);

    await new Promise<void>((resolve, reject) => {
      const audio = new Audio(url);
      this.#cloudAudio = audio;

      audio.onended = () => {
        URL.revokeObjectURL(url);
        this.#cloudAudio = null;
        resolve();
      };
      audio.onerror = () => {
        URL.revokeObjectURL(url);
        this.#cloudAudio = null;
        reject(new Error("音频播放失败"));
      };
      audio.play().catch(reject);
    });
  }
}

export const ttsController = new TtsController();
