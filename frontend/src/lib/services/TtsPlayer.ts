import type {TtsQueueItem} from "$types/TtsQueueItem";
import {tts} from "$services/WebViewTtsService.svelte.js";

let activeAbort: AbortController | null = null;

/** 播放单个已就绪的 Item，支持外部中断。完成后自动清理 URL。 */
export function play(item: TtsQueueItem, abortSignal: AbortSignal): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    if (abortSignal.aborted) {
      reject(new DOMException("Aborted", "AbortError"));
      return;
    }

    const onAbort = () => {
      if (item.generatedMode === "SystemNative") {
        tts.cancel();
      } else {
        audio?.pause();
        audio?.removeAttribute("src");
        audio = null;
      }
      reject(new DOMException("Aborted", "AbortError"));
    };
    abortSignal.addEventListener("abort", onAbort, {once: true});

    let audio: HTMLAudioElement | null = null;

    const cleanup = () => {
      abortSignal.removeEventListener("abort", onAbort);
      if (item.audioUrl) URL.revokeObjectURL(item.audioUrl);
    };

    const onEnd = () => {
      cleanup();
      resolve();
    };

    const onErr = (msg: string) => {
      cleanup();
      reject(new Error(msg));
    };

    switch (item.generatedMode) {
      case "SystemNative": {
        tts
          .play(item.name)
          .then(onEnd)
          .catch((e: unknown) => {
            if (abortSignal.aborted) return; // abort 已处理
            onErr(e instanceof Error ? e.message : String(e));
          });
        break;
      }

      case "AICloud":
      case "AIHttp":
      case "AIEmbedded": {
        if (!item.audioUrl) {
          cleanup();
          onErr("音频 URL 不存在");
          return;
        }
        audio = new Audio(item.audioUrl);
        audio.onended = onEnd;
        audio.onerror = () => onErr("音频播放失败");
        audio.play().catch((e: unknown) => {
          if (abortSignal.aborted) return;
          onErr(e instanceof Error ? e.message : String(e));
        });
        break;
      }
    }
  });
}

/** 强制中断当前活跃播放（由 speakNow / cancel / effect cleanup 调用） */
export function abortCurrent() {
  activeAbort?.abort();
  activeAbort = null;
}

/** 内部使用：创建新的 AbortController 并返回，用于 Scheduler 的 effect 中 */
export function beginPlay(item: TtsQueueItem): { signal: AbortSignal; done: Promise<void> } {
  abortCurrent(); // 确保旧的被清理
  const ac = new AbortController();
  activeAbort = ac;
  const done = play(item, ac.signal).finally(() => {
    if (activeAbort === ac) activeAbort = null;
  });
  return {signal: ac.signal, done};
}
