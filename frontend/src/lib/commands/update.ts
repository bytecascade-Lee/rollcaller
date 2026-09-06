import {invoke} from "@tauri-apps/api/core";
import type {UpdateView} from "$types/UpdateView";

/** 展示视图广播事件名（与后端 `cmd::update::UPDATE_VIEW_EVENT` 一致） */
export const UPDATE_VIEW_EVENT = "update://view";

/**
 * 更新命令统一封装：所有命令返回裁剪的展示视图 [`UpdateView`]。
 * 业务失败折叠为视图的 `error` 变体；`Err` 仅用于防重入/环境拒绝，
 * 前端捕获后直接提示即可（不影响 store 视图）。
 */

/** 检查是否有可用更新 */
export async function check(): Promise<UpdateView> {
  return await invoke<UpdateView>("check");
}

/** 下载已批准产物（进度经后端广播实时上报） */
export async function download(): Promise<UpdateView> {
  return await invoke<UpdateView>("download");
}

/** 取消下载 / 放弃已下载产物 */
export async function cancel(): Promise<UpdateView> {
  return await invoke<UpdateView>("cancel");
}

/** 安装已下载产物（成功路径进程退出，安装器接管） */
export async function install(): Promise<UpdateView> {
  return await invoke<UpdateView>("install");
}

/** 查询当前展示视图（页面挂载初始化用） */
export async function state(): Promise<UpdateView> {
  return await invoke<UpdateView>("state");
}
