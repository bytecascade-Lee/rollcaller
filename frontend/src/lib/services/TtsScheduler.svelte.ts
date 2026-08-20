import {ttsStore as store} from "$stores/TtsStore.svelte.js";
import {loadItem} from "./TtsLoader";
import {abortCurrent, beginPlay} from "./TtsPlayer";

let initialized = false;
/** 当前正在异步加载的 item id（防止重复触发加载） */
let loadingId: string | null = null;

/**
 * 队列泵：驱动 "加载 → 播放" 流水线。
 *
 * 由入队 / 加载完成 / 播放完成 / 恢复播放时显式调用（pumpQueue），
 * 不使用 $effect 轮询 —— effect 内写入队列状态（status: "Playing"）
 * 会触发 Svelte 重跑自身，重跑前先执行 cleanup（abortCurrent），
 * 导致刚启动的播放被立即中断、item 卡死在 "Playing" 阻塞整个队列。
 */
function pump() {
  if (!initialized) return;

  // 1. 加载第一个 Loading 项（与播放并行推进）
  const pending = store.items.find((i) => i.status === "Loading");
  if (pending && loadingId !== pending.id) {
    loadingId = pending.id;
    loadItem(store, pending).finally(() => {
      if (loadingId === pending.id) loadingId = null;
      pump();
    });
  }

  // 2. 正在播放或暂停 → 等待下一次触发
  if (store.currentId !== null || store.isPaused) return;

  // 3. 播放第一个 Ready 项
  const ready = store.items.find((i) => i.status === "Ready");
  if (!ready) return;

  store.currentId = ready.id;
  store.update(ready.id, {status: "Playing"});
  const {done} = beginPlay(ready);

  done
    .then(() => {
      store.remove(ready.id);
    })
    .catch((e: unknown) => {
      if (e instanceof DOMException && e.name === "AbortError") return;
      const msg = e instanceof Error ? e.message : String(e);
      console.error("TTS playback error:", msg);
      store.update(ready.id, {status: "Done", error: msg});
      store.remove(ready.id);
    })
    .finally(() => {
      if (store.currentId === ready.id) store.currentId = null;
      pump();
    });
}

/** 初始化调度器（组件挂载时调用） */
export function initScheduler() {
  initialized = true;
}

/** 销毁调度器（组件卸载时调用）：中断播放并停止泵 */
export function destroyScheduler() {
  abortCurrent();
  loadingId = null;
  initialized = false;
}

/** 队列发生变化时由外部驱动一次泵（入队 / 恢复播放） */
export function pumpQueue() {
  pump();
}
