import {TtsQueueItem} from "$types/TtsQueueItem";
import {TtsMode} from "$types/TtsMode";

export class TtsStore {
  #items = $state<TtsQueueItem[]>([]);
  #mode = $state<TtsMode>("SystemNative");
  #isPaused = $state(false);
  #currentId = $state<string | null>(null);

  get items() {
    return this.#items;
  }

  get mode() {
    return this.#mode;
  }

  set mode(value: TtsMode) {
    this.#mode = value;
  }

  get isSpeaking() {
    return this.#currentId !== null && !this.#isPaused;
  }

  get isPaused() {
    return this.#isPaused;
  }

  set isPaused(v: boolean) {
    this.#isPaused = v;
  }

  get currentId(): string | null {
    return this.#currentId;
  }

  set currentId(id: string | null) {
    this.#currentId = id;
  }

  /** 队列头部第一个 Item（供 Scheduler 的 $derived 消费） */
  get firstItem(): TtsQueueItem | undefined {
    return this.#items[0];
  }

  /** 队列长度（供 Scheduler 的 $derived 消费，避免依赖整个数组引用） */
  get length() {
    return this.#items.length;
  }

  /** 入队 — 自动注入当前全局 mode 为 generatedMode 快照 */
  add(item: Omit<TtsQueueItem, "generatedMode">) {
    const full: TtsQueueItem = {...item, generatedMode: this.#mode};
    this.#items = [...this.#items, full];
  }

  update(id: string, updates: Partial<TtsQueueItem>) {
    this.#items = this.#items.map((item) =>
      item.id === id ? {...item, ...updates} : item,
    );
  }

  remove(id: string) {
    this.#items = this.#items.filter((item) => item.id !== id);
  }

  /**
   * 清空队列。
   * 只释放 **非当前播放** 的 URL；当前播放的 URL 由 Player 的 finally 块释放，
   * 防止二次 revokeObjectURL。
   */
  clearAll() {
    for (const item of this.#items) {
      if (item.id !== this.#currentId && item.audioUrl) {
        URL.revokeObjectURL(item.audioUrl);
      }
    }
    this.#items = [];
    this.#currentId = null;
  }

  nextMode() {
    switch (this.#mode) {
      case "Off":
        this.#mode = "SystemNative"
        break;
      case "SystemNative":
        this.#mode = "AICloud";
        break;
      case "AICloud":
        this.#mode = "Off";
    }
  }
}

export const ttsStore = new TtsStore();
