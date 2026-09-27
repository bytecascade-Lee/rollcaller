/**
 * 跨窗口导航命令
 */
import {invoke} from "@tauri-apps/api/core";
import type {Action, Payload, Route} from "$routing/address";

/** 唤醒事件名 */
export const NAV_WAKE_EVENT = "rollcaller://nav/wake";

/** 后端搬过来的待取载荷 */
export type PendingNav = {
  route: Route;
  payload?: Payload | null;
};

/** 投递 */
export async function navOpen(action: Action): Promise<void> {
  await invoke<void>("nav_open", {
    route: action.route,
    payload: action.payload ?? null,
  });
}

/** 取走并清空本窗口的待取载荷 */
export async function navTakePending(): Promise<PendingNav | null> {
  return await invoke<PendingNav | null>("nav_take_pending");
}
