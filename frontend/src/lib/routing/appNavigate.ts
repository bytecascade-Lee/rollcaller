import {type Action, type Route, routeOf} from "$routing/address";
import {
  APP_ENTRY_PARENT,
  APP_HOST,
  type AppEntryId,
  type AppEntryPayload,
} from "$routing/app.entries.generated";
import {nav} from "$routing/nav";
import {chainOf} from "$routing/entries";

/*
 * app 域的导航门面：入口 ↔ 地址的**唯一**翻译者。
 *
 * 只翻译与投递，不落地 —— 地址落到哪个浮层由窗口根组件的转换语义决定（见 `App.svelte`）。
 *
 * 形态是 `Object.freeze` 的命名空间而不是 class：这里**没有状态**，也没有一个方法读 `this`。
 * 用 class 会让读者以为它有生命周期。有状态的门面（`helpNavigate` 的栈）才用 class。
 */

/** 入口地址 */
function path(entry: AppEntryId): Route {
  return routeOf(APP_HOST, chainOf(entry));
}

/** 入口 + 载荷 → 可投递的 Action */
function action<E extends AppEntryId>(entry: E, payload?: AppEntryPayload[E]): Action {
  return {route: path(entry), payload};
}

/**
 * **进入**一个入口 —— app 域唯一的"打开"入口。
 *
 * 载荷的类型由 `meta.json` 的声明决定（生成 `AppEntryPayload`），写错形状即编译报错；
 * 但**允许省略**：地址本来就可以没有载荷（深链就是只有地址），此时浮层自行降级为不打开。
 */
function enter<E extends AppEntryId>(entry: E, payload?: AppEntryPayload[E]): void {
  nav.go(action(entry, payload));
}

/** **离开**一个入口：地址回退到它的父级 —— app 域唯一的"关闭"入口 */
function leave(entry: AppEntryId): void {
  // 只在自己**确实是当前地址的目标**时才回退。否则说明别处已经在导航 —— 典型场景：
  // 浮层锚定在工具栏上，用户点侧边栏切卡片，侧边栏的导航先落地（冒泡顺序），随后
  // 浮层的 clickOutside 才触发；若这里再推一次，就会把刚切过去的卡片拽回来。
  if (nav.currentRoute() !== path(entry)) {
    return;
  }
  const parent = APP_ENTRY_PARENT[entry];
  if (parent !== null) {
    nav.go(action(parent));
  }
}

export const appNavigate = Object.freeze({path, action, enter, leave});
