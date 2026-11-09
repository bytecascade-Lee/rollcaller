import {invoke} from "@tauri-apps/api/core";
import type {UpdateView} from "$types/UpdateView";

/** 展示视图广播事件名 */
export const UPDATE_VIEW_EVENT = "rollcaller://update/view";

/** 下载进度窄帧事件名 */
export const DOWNLOAD_PROGRESS_EVENT = "rollcaller://update/download";

/** 检查是否有可用更新 */
export async function check(): Promise<UpdateView> {
  return await invoke<UpdateView>("updater_check");
}

/** 下载已批准产物 */
export async function download(): Promise<UpdateView> {
  return await invoke<UpdateView>("updater_download");
}

/** 取消下载 / 放弃已下载产物 */
export async function cancel(): Promise<UpdateView> {
  return await invoke<UpdateView>("updater_cancel");
}

/** 安装已下载产物 */
export async function install(): Promise<UpdateView> {
  return await invoke<UpdateView>("updater_install");
}

/** 查询当前展示视图 */
export async function state(): Promise<UpdateView> {
  return await invoke<UpdateView>("updater_current_state");
}
