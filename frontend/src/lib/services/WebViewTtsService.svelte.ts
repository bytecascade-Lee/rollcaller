/**
 * WebViewTtsService — 纯本地浏览器 TTS（Web Speech API）
 *
 * 职责单一：管理语音队列、播报、暂停、恢复、取消。
 * 不涉及云端逻辑，由 TtsController 协调调用。
 */
class WebViewTtsService {
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

  /** 追加到队列 */
  speak(text: string, options?: { lang?: string; rate?: number; pitch?: number }) {
    if (!text?.trim()) return;
    this.#queue.push(text.trim());
    if (!this.#isSpeaking) {
      this.#processNext(options);
    }
  }

  /** 打断当前，清空队列，立即播报 */
  speakNow(text: string, options?: { lang?: string; rate?: number; pitch?: number }) {
    if (!text?.trim()) return;
    this.#synthesis.cancel();
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
    this.#synthesis.pause();
    this.#isPaused = true;
  }

  resume() {
    if (!this.#isPaused) return;
    this.#synthesis.resume();
    this.#isPaused = false;
  }

  cancel() {
    this.#synthesis.cancel();
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
}

export const tts = new WebViewTtsService();
