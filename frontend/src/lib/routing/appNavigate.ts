import {type Action, type Payload, routeOf} from "$routing/address";
import {APP_ENTRY_PARENT, APP_HOST, type AppEntryId} from "$routing/entries.generated";
import {WindowRouter} from "$routing/windowRouter";
import {chainOf} from "$routing/entries";

/**
 * app 域的导航：入口 ↔ 地址的**唯一**翻译者。
 *
 * 只翻译与投递，不落地 —— 地址落到哪个浮层由窗口根组件的转换语义决定（见 `App.svelte`）。
 * 单例：窗口只有一份地址，调用方不必各自认得路由对象。
 */
class AppNavigate {
  /** 入口地址 */
  path(entry: AppEntryId): string {
    return routeOf(APP_HOST, chainOf(entry));
  }

  /** 入口 + 载荷 → 可投递的 Action */
  action(entry: AppEntryId, payload?: Payload): Action {
    return {route: this.path(entry), payload};
  }

  /** **进入**一个入口 —— app 域唯一的"打开"入口 */
  enter(entry: AppEntryId, payload?: Payload): void {
    WindowRouter.nav(this.action(entry, payload));
  }

  /** **离开**一个入口：地址回退到它的父级 —— app 域唯一的"关闭"入口 */
  leave(entry: AppEntryId): void {
    // 只在自己**确实是当前地址的目标**时才回退。否则说明别处已经在导航 —— 典型场景：
    // 浮层锚定在工具栏上，用户点侧边栏切卡片，侧边栏的导航先落地（冒泡顺序），随后
    // 浮层的 clickOutside 才触发；若这里再推一次，就会把刚切过去的卡片拽回来。
    if (WindowRouter.currentRoute() !== this.path(entry)) {
      return;
    }
    const parent = APP_ENTRY_PARENT[entry];
    if (parent !== null) {
      WindowRouter.nav(this.action(parent));
    }
  }
}

export const appNavigate = new AppNavigate();
