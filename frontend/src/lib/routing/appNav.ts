import {APP_ENTRY_PARENT, APP_HOST, AppEntryId} from "$routing/entries.generated";
import {Action, Payload, routeOf} from "$routing/address";
import {WindowRouter} from "$routing/windowRouter";
import {chainOf} from "$routing/entries";

/** 入口地址 */
export function appPath(entry: AppEntryId): string {
  return routeOf(APP_HOST, chainOf(entry));
}

/** 入口 + 载荷 → 可投递的 Action */
export function appAction(entry: AppEntryId, payload?: Payload): Action {
  return {route: appPath(entry), payload};
}

/**
 * **进入**一个入口 —— app 域唯一的"打开"入口。
 *
 */
export function appEnter(entry: AppEntryId, payload?: Payload): void {
  WindowRouter.nav(appAction(entry, payload));
}

/**
 * **离开**一个入口：地址回退到它的父级 —— app 域唯一的"关闭"入口。
 *
 */
export function appLeave(entry: AppEntryId): void {
  // 只在自己**确实是当前地址的目标**时才回退。否则说明别处已经在导航 —— 典型场景：
  // 浮层锚定在工具栏上，用户点侧边栏切卡片，侧边栏的导航先落地（冒泡顺序），随后
  // 浮层的 clickOutside 才触发；若这里再推一次，就会把刚切过去的卡片拽回来。
  if (WindowRouter.currentRoute() !== appPath(entry)) {
    return;
  }
  const parent = APP_ENTRY_PARENT[entry];
  if (parent !== null) {
    WindowRouter.nav(appAction(parent));
  }
}
