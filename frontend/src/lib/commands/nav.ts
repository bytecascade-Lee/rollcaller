/**
 * 跨窗口导航命令（薄壳）。
 *
 * 只有两个动作：**投递**与**取走**。整套东西只有一条数据通路（后端单槽 pending），
 * 事件只负责"叫醒"——所以不存在"事件带的载荷"和"取走的载荷"两份真相。
 *
 * 关于"就绪"：不需要额外的 ready 命令。冷启动时窗口不存在、后端**不发事件**，
 * 载荷静静躺在 pending 里；目标前端挂载后自己会 `nav_take_pending()` 一次——
 * **这次取走本身就是就绪信号**（一个动作同时表达了"我起来了"和"把东西给我"）。
 */
import {invoke} from "@tauri-apps/api/core";
import type {Action, Payload, Route} from "$routing/route";

/** 唤醒事件名：**不带载荷**，只是"有新东西，来取" */
export const NAV_WAKE_EVENT = "rollcaller://nav/wake";

/** 后端搬过来的待取载荷（后端不解释 route / payload，只原样搬运） */
export type PendingNav = {
  route: Route;
  payload?: Payload | null;
};

/**
 * 投递：目标窗口名由**后端**从 `route` 第一段推出（"域即窗口名"那条隐含规则），
 * 所以这里不传 target——少一个能与 route 自相矛盾的入参。
 *
 * 窗口不存在后端会建窗（此时不发事件）；已存在则先唤到前台再发唤醒事件。
 */
export async function navOpen(action: Action): Promise<void> {
  await invoke<void>("nav_open", {
    route: action.route,
    payload: action.payload ?? null,
  });
}

/** 取走并清空**本窗口**的待取载荷（host 由后端从调用窗口的 label 取，前端传不了谎） */
export async function navTakePending(): Promise<PendingNav | null> {
  return await invoke<PendingNav | null>("nav_take_pending");
}
