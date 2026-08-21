/**
 * WebViewTtsService — 无状态的本地 TTS 播放器
 *
 * 职责单一：接收文本 → 播放 → 完成/失败。
 * 队列、调度、模式切换全部由 TtsController 管理。
 */
class WebViewTtsService {
  #synthesis = window.speechSynthesis;
  #voicesReady = $state(false);
  #voicesReadyPromise: Promise<void> | null = null;
  #currentUtterance: SpeechSynthesisUtterance | null = null;

  constructor() {
    this.#initVoices();
  }

  #initVoices() {
    this.#voicesReadyPromise = new Promise<void>((resolve) => {
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

  /**
   * 播放文本，返回 Promise —— onend resolve，onerror reject
   */
  play(text: string, options?: { lang?: string; rate?: number; pitch?: number }): Promise<void> {
    if (!text?.trim()) return Promise.resolve();

    return new Promise<void>((resolve, reject) => {
      this.#ensureVoicesReady().then(() => {
        const utterance = new SpeechSynthesisUtterance(text);
        this.#currentUtterance = utterance;

        utterance.lang = options?.lang || "zh-CN";
        utterance.rate = options?.rate || 1.0;
        utterance.pitch = options?.pitch || 1.0;

        const voices = this.#synthesis.getVoices();
        const zhVoice = voices.find((v) => v.lang.startsWith("zh"));
        if (zhVoice) utterance.voice = zhVoice;

        utterance.onend = () => {
          this.#currentUtterance = null;
          resolve();
        };

        utterance.onerror = (event) => {
          this.#currentUtterance = null;
          if (event.error === "canceled") {
            resolve(); // cancel 不算错误
            return;
          }
          reject(new Error(`TTS Error: ${event.error}`));
        };

        this.#synthesis.speak(utterance);
      });
    });
  }

  /** 立即停止当前播放（不触发 reject） */
  cancel() {
    this.#synthesis.cancel();
    this.#currentUtterance = null;
  }

  pause() {
    this.#synthesis.pause();
  }

  resume() {
    this.#synthesis.resume();
  }
}

export const tts = new WebViewTtsService();
