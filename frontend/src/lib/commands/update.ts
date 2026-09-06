import {Channel, invoke} from "@tauri-apps/api/core";
import type {UpdateState} from "$types/UpdateState";

/** 下载进度事件（与后端 `DownloadEvent` 契约一致） */
export type DownloadEvent =
  | { event: "Started"; data: { contentLength: number | null } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

/** 下载进度回调（后端返回累计值换算后的本块增量；由命令层累加为已下载量） */
export type DownloadProgressFn = (downloaded: number, total: number | null) => void;

/** 检查是否有可用更新（后端推进状态，返回最新快照） */
export async function check(): Promise<UpdateState> {
  return await invoke<UpdateState>("update_check");
}

/**
 * 下载已批准产物（进度经 Channel 上报，结束后返回最终快照）。
 * `onProgress` 可选：不传则进度只由返回快照校准。
 */
export async function download(onProgress?: DownloadProgressFn): Promise<UpdateState> {
  const channel = new Channel<DownloadEvent>();
  let downloaded = 0;
  let total: number | null = null;
  channel.onmessage = (e) => {
    switch (e.event) {
      case "Started":
        total = e.data.contentLength ?? null;
        onProgress?.(0, total);
        break;
      case "Progress":
        downloaded += e.data.chunkLength;
        onProgress?.(downloaded, total);
        break;
      case "Finished":
        break;
    }
  };
  return await invoke<UpdateState>("update_download", {onEvent: channel});
}

/** 取消下载 / 放弃已下载产物（返回最新快照） */
export async function cancel(): Promise<UpdateState> {
  return await invoke<UpdateState>("update_cancel_download");
}

/** 安装已下载产物（成功路径进程退出，安装器接管） */
export async function install(): Promise<UpdateState> {
  return await invoke<UpdateState>("update_install");
}

/** 查询当前状态快照（页面挂载初始化用） */
export async function state(): Promise<UpdateState> {
  return await invoke<UpdateState>("update_state");
}
