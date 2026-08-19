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

      // 如果语音已经加载完成
      if (this.#synthesis.getVoices().length > 0) {
        this.#voicesReady = true;
        resolve();
        return;
      }

      // 监听语音加载事件
      const handler = () => {
        this.#voicesReady = true;
        resolve();
        this.#synthesis.removeEventListener('voiceschanged', handler);
      };
      this.#synthesis.addEventListener('voiceschanged', handler);

      // 超时保护：1秒后如果还没加载完成也继续
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
   * 追加文本到队列末尾
   */
  speak(text: string, options?: { lang?: string; rate?: number; pitch?: number }) {
    if (!text?.trim()) return;

    // 追加到队尾
    this.#queue.push(text.trim());

    // 如果当前空闲，开始播报
    if (!this.#isSpeaking) {
      this.#processNext(options);
    }
  }

  /**
   * 打断当前播报，清空队列，立即播报新内容
   */
  speakNow(text: string, options?: { lang?: string; rate?: number; pitch?: number }) {
    if (!text?.trim()) return;

    // 取消当前播报
    this.#synthesis.cancel();

    // 清空队列
    this.#queue = [];

    // 重置状态
    this.#isSpeaking = false;
    this.#isPaused = false;
    this.#retryCount = 0;
    this.#currentUtterance = null;

    // 追加到队列并立即播报
    this.#queue.push(text.trim());
    this.#processNext(options);
  }

  /**
   * 暂停当前播报
   */
  pause() {
    if (!this.#isSpeaking) return;

    this.#synthesis.pause();
    this.#isPaused = true;
  }

  /**
   * 恢复暂停的播报
   */
  resume() {
    if (!this.#isPaused) return;

    this.#synthesis.resume();
    this.#isPaused = false;
  }

  /**
   * 停止播报并清空队列
   */
  cancel() {
    this.#synthesis.cancel();
    this.#queue = [];
    this.#isSpeaking = false;
    this.#isPaused = false;
    this.#currentText = null;
    this.#retryCount = 0;
    this.#currentUtterance = null;
  }

  /**
   * 清空队列（不影响当前播报）
   */
  clearQueue() {
    this.#queue = [];
  }

  #processNext(options?: { lang?: string; rate?: number; pitch?: number }) {
    // 如果队列为空，回到空闲状态
    if (this.#queue.length === 0) {
      this.#isSpeaking = false;
      this.#isPaused = false;
      this.#currentText = null;
      this.#retryCount = 0;
      this.#currentUtterance = null;
      return;
    }

    // 如果处于暂停状态，不播放下一条
    if (this.#isPaused) {
      return;
    }

    // 取出第一条
    const text = this.#queue[0];
    this.#currentText = text;
    this.#isSpeaking = true;

    // 确保语音就绪
    this.#ensureVoicesReady().then(() => {
      // 再次检查队列（可能在等待过程中被取消）
      if (this.#queue.length === 0 || this.#currentText !== text) {
        return;
      }

      // 创建 utterance
      const utterance = new SpeechSynthesisUtterance(text);
      this.#currentUtterance = utterance;

      // 配置参数
      utterance.lang = options?.lang || 'zh-CN';
      utterance.rate = options?.rate || 1.0;
      utterance.pitch = options?.pitch || 1.0;

      // 选择中文语音
      const voices = this.#synthesis.getVoices();
      const zhVoice = voices.find(v => v.lang.startsWith('zh'));
      if (zhVoice) utterance.voice = zhVoice;

      // 事件绑定
      utterance.onend = () => {
        // 播报完成，从队列移除
        if (this.#queue.length > 0 && this.#queue[0] === text) {
          this.#queue.shift();
        }
        this.#retryCount = 0;
        this.#currentUtterance = null;
        // 播放下一条
        this.#processNext(options);
      };

      utterance.onerror = (event) => {
        // 如果是被取消导致的错误，不处理
        if (event.error === 'canceled') {
          this.#currentUtterance = null;
          return;
        }

        console.error('TTS Error:', event.error, 'Text:', text);

        // 重试逻辑
        if (this.#retryCount < this.#maxRetries) {
          this.#retryCount++;
          // 延迟重试，退避策略：200ms * retryCount
          setTimeout(() => {
            // 检查是否被取消或状态变化
            if (this.#currentUtterance === utterance && !this.#isPaused) {
              this.#processNext(options);
            }
          }, 200 * this.#retryCount);
        } else {
          // 重试次数用完，跳过该条
          console.warn('TTS: Max retries reached, skipping:', text);
          if (this.#queue.length > 0 && this.#queue[0] === text) {
            this.#queue.shift();
          }
          this.#retryCount = 0;
          this.#currentUtterance = null;
          // 继续播放下一条
          this.#processNext(options);
        }
      };

      // 开始播报
      this.#synthesis.speak(utterance);
    });
  }
}

export const tts = new TTSService();
