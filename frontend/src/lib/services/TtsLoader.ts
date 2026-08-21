import type {TtsQueueItem} from "$types/TtsQueueItem";
import type {TtsStore} from "$stores/TtsStore.svelte.js";
import {TtsCommand} from "$commands";

/**
 * 异步资源加载器。
 *
 * 根据 item.generatedMode 将 Loading 状态的 Item 转为 Ready（或 Done+error）。
 * 不关心播放时机，仅负责资源就绪。
 */
export async function loadItem(store: TtsStore, item: TtsQueueItem): Promise<void> {
  // 已经不是 Loading 了（可能被 speakNow 清空），直接返回
  if (item.status !== "Loading") return;

  try {
    switch (item.generatedMode) {
      case "SystemNative":
        store.update(item.id, {status: "Ready"});
        break;

      case "AICloud":
      case "AIHttp":
      case "AIEmbedded": {
        const b64 = await TtsCommand.speak(item.name);
        // 校验：item 可能已被 clearAll 移除
        if (!store.items.find((i) => i.id === item.id)) return;
        const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
        const blob = new Blob([bytes], {type: "audio/wav"});
        const audioUrl = URL.createObjectURL(blob);
        store.update(item.id, {status: "Ready", audioUrl});
        break;
      }
    }
  } catch (e: unknown) {
    if (!store.items.find((i) => i.id === item.id)) return;
    const msg = e instanceof Error ? e.message : String(e);
    console.error("TTS load error:", msg);
    store.update(item.id, {status: "Done", error: msg});
  }
}
