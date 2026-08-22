import {ttsStore} from "$stores/TtsStore.svelte.js";
import {tts} from "$services/WebViewTtsService.svelte";
import {abortCurrent} from "$services/TtsPlayer";
import {pumpQueue} from "$services/TtsScheduler.svelte.js";

/**
 * TTS 业务命令入口。
 *
 * 仅对外暴露 speak / speakNow / pause / resume / cancel。
 * 资源加载与播放调度由 ttsScheduler 的队列泵显式驱动（pumpQueue）。
 */
class TtsController {
  /** 当前播报文本 */
  get currentText(): string | null {
    if (!ttsStore.currentId) return null;
    return ttsStore.items.find((i) => i.id === ttsStore.currentId)?.name ?? null;
  }

  /** 追加到队列（队列泵自动触发加载与播放） */
  speak(name: string) {
    ttsStore.add({id: crypto.randomUUID(), name, status: "Loading"});
    pumpQueue();
  }

  /** 打断当前 → 清空队列 → 立即播报新内容 */
  speakNow(name: string) {
    abortCurrent();
    ttsStore.clearAll();
    this.speak(name);
  }

  /** 暂停 */
  pause() {
    ttsStore.isPaused = true;
    if (ttsStore.mode === "SystemNative") tts.pause();
  }

  /** 恢复 */
  resume() {
    ttsStore.isPaused = false;
    if (ttsStore.mode === "SystemNative") tts.resume();
    pumpQueue();
  }

  /** 停止并清空队列 */
  cancel() {
    abortCurrent();
    ttsStore.clearAll();
    tts.cancel();
  }
}

export const ttsController = new TtsController();
