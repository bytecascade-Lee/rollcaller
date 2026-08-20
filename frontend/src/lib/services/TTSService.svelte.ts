import { TTSCommand } from "$commands";

/** TTS 模式：local = 浏览器本地合成，cloud = 云端大模型 */
export type TTSMode = "local" | "cloud";

/** 全局 TTS 模式状态，供组件绑定 */
export const ttsMode = $state<{ value: TTSMode }>({ value: "local" });

class TTSService {
  #synthesis = window.speechSynthesis;
  #queue = $state<string[]>([]);
  #isSpeaking = $state(false);
  #isPaused = $state(false);
  #currentText = $state<string | null>(null);
  #retryCount = 0;
  #maxRetries = 2;
  #voicesReady = $state(false);
  #voicesReadyPromise: Promise<void> | null = null;
  #resolveVoicesReady: (() => void) | null = null;
  #currentUtterance: SpeechSynthesisUtterance | null = null;

  /** cloud 模式：当前播放的 Audio 元素 */
  #cloudAudio: HTMLAudioElement | null = null;
  /** cloud 模式：等待后端返回的音频数据后立即播放 */
  #cloudSkip = false;

  get speaking() {
    return this.#isSpeaking;
  }

  get paused() {
    return this.#isPaused;
  }

  get currentText() {
    return this.#currentText;
  }

  get queueLength() {
    return this.#queue.length;
  }

  get hasQueue() {
    return this.#queue.length > 0;
  }

  get mode(): TTSMode {
    return ttsMode.value;
  }

  set mode(v: TTSMode) {
    ttsMode.value = v;
  }

  constructor() {
    this.#initVoices();
  }

  #initVoices() {
    this.#voicesReadyPromise = new Promise<void>((resolve) => {
      this.#resolveVoicesReady = resolve;
      if (this.#synthesis.getVoices().length > 0) {
        this.#voicesReady = true;
        resolve();
        return;
      }
      const handler = () => {
        this.#voicesReady = true;
        resolve();
        this.#synthesis.removeEventListener("voiceschanged", handler);
      };
      this.#synthesis.addEventListener("voiceschanged", handler);
      setTimeout(() => {
        if (!this.#voicesReady) {
          this.#voicesReady = true;
          resolve();
        }
      }, 1000);
    });
  }

  #ensureVoicesReady() {
    return this.#voicesReadyPromise || Promise.resolve();
  }

  // ─── 公共 API ───────────────────────────────────────

  /**
   * 追加文本到队列末尾
   */
  speak(text: string, options?: { lang?: string; rate?: number; pitch?: number }) {
    if (!text?.trim()) return;
    this.#queue.push(text.trim());
    if (!this.#isSpeaking) {
      this.#processNext(options);
    }
  }

  /**
   * 打断当前播报，清空队列，立即播报新内容
   */
  speakNow(text: string, options?: { lang?: string; rate?: number; pitch?: number }) {
    if (!text?.trim()) return;

    // 打断本地
    this.#synthesis.cancel();
    // 打断云端
    if (this.#cloudAudio) {
      this.#cloudAudio.pause();
      this.#cloudAudio.src = "";
      this.#cloudAudio = null;
    }
    this.#cloudSkip = true;

    this.#queue = [];
    this.#isSpeaking = false;
    this.#isPaused = false;
    this.#retryCount = 0;
    this.#currentUtterance = null;

    this.#queue.push(text.trim());
    this.#processNext(options);
  }

  pause() {
    if (!this.#isSpeaking) return;
    if (this.#cloudAudio) {
      this.#cloudAudio.pause();
    } else {
      this.#synthesis.pause();
    }
    this.#isPaused = true;
  }

  resume() {
    if (!this.#isPaused) return;
    if (this.#cloudAudio) {
      this.#cloudAudio.play();
    } else {
      this.#synthesis.resume();
    }
    this.#isPaused = false;
  }

  cancel() {
    this.#synthesis.cancel();
    if (this.#cloudAudio) {
      this.#cloudAudio.pause();
      this.#cloudAudio.src = "";
      this.#cloudAudio = null;
    }
    this.#cloudSkip = false;
    this.#queue = [];
    this.#isSpeaking = false;
    this.#isPaused = false;
    this.#currentText = null;
    this.#retryCount = 0;
    this.#currentUtterance = null;
  }

  clearQueue() {
    this.#queue = [];
  }

  // ─── 路由：根据模式分发 ────────────────────────────

  #processNext(options?: { lang?: string; rate?: number; pitch?: number }) {
    if (this.#queue.length === 0) {
      this.#isSpeaking = false;
      this.#isPaused = false;
      this.#currentText = null;
      this.#retryCount = 0;
      this.#currentUtterance = null;
      return;
    }
    if (this.#isPaused) return;

    if (ttsMode.value === "cloud") {
      this.#processNextCloud();
    } else {
      this.#processNextLocal(options);
    }
  }

  // ─── 本地模式（Web Speech API）──────────────────────

  #processNextLocal(options?: { lang?: string; rate?: number; pitch?: number }) {
    if (this.#queue.length === 0 || this.#isPaused) return;

    const text = this.#queue[0];
    this.#currentText = text;
    this.#isSpeaking = true;

    this.#ensureVoicesReady().then(() => {
      if (this.#queue.length === 0 || this.#currentText !== text) return;

      const utterance = new SpeechSynthesisUtterance(text);
      this.#currentUtterance = utterance;

      utterance.lang = options?.lang || "zh-CN";
      utterance.rate = options?.rate || 1.0;
      utterance.pitch = options?.pitch || 1.0;

      const voices = this.#synthesis.getVoices();
      const zhVoice = voices.find((v) => v.lang.startsWith("zh"));
      if (zhVoice) utterance.voice = zhVoice;

      utterance.onend = () => {
        if (this.#queue.length > 0 && this.#queue[0] === text) {
          this.#queue.shift();
        }
        this.#retryCount = 0;
        this.#currentUtterance = null;
        this.#processNext(options);
      };

      utterance.onerror = (event) => {
        if (event.error === "canceled") {
          this.#currentUtterance = null;
          return;
        }
        console.error("TTS Error:", event.error, "Text:", text);
        if (this.#retryCount < this.#maxRetries) {
          this.#retryCount++;
          setTimeout(() => {
            if (this.#currentUtterance === utterance && !this.#isPaused) {
              this.#processNext(options);
            }
          }, 200 * this.#retryCount);
        } else {
          console.warn("TTS: Max retries reached, skipping:", text);
          if (this.#queue.length > 0 && this.#queue[0] === text) {
            this.#queue.shift();
          }
          this.#retryCount = 0;
          this.#currentUtterance = null;
          this.#processNext(options);
        }
      };

      this.#synthesis.speak(utterance);
    });
  }

  // ─── 云端模式（后端 API + 浏览器播放）─────────────

  async #processNextCloud() {
    if (this.#queue.length === 0 || this.#isPaused) return;

    const text = this.#queue[0];
    this.#currentText = text;
    this.#isSpeaking = true;
    this.#cloudSkip = false;

    try {
      // 1. 后端：缓存/获取音频
      await TTSCommand.speak(text);

      // 2. 被 speakNow/cancel 打断，跳过本次播放
      if (this.#cloudSkip || this.#queue.length === 0 || this.#currentText !== text) {
        this.#cloudSkip = false;
        if (this.#queue.length === 0) {
          this.#isSpeaking = false;
          this.#currentText = null;
        }
        return;
      }

      // 3. 获取 Base64 音频并播放
      const b64 = await TTSCommand.getAudio(text);
      const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
      const blob = new Blob([bytes], { type: "audio/wav" });
      const url = URL.createObjectURL(blob);

      const audio = new Audio(url);
      this.#cloudAudio = audio;

      await new Promise<void>((resolve, reject) => {
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

      // 播放完成，移除队列首项
      if (this.#queue.length > 0 && this.#queue[0] === text) {
        this.#queue.shift();
      }
      this.#processNext();
    } catch (e) {
      console.error("Cloud TTS Error:", e);
      // 跳过失败项
      if (this.#queue.length > 0 && this.#queue[0] === text) {
        this.#queue.shift();
      }
      this.#cloudAudio = null;
      this.#processNext();
    }
  }
}

export const tts = new TTSService();
