import {type Action, type Route, routeOf} from "$routing/address";
import {APP_ENTRY_PARENT, APP_HOST, type AppEntryId, type AppEntryPayload} from "$routing/app/entries.generated";
import {chainOf} from "$routing/app/entries";
import {nav} from "$routing/nav";

/**
 * app 域的门面：入口 ↔ 地址的唯一翻译者。只翻译与投递，落地在 `appDomain`。
 *
 * 用 `Object.freeze` 的命名空间而非 class：这里没有状态，也没有方法读 `this`
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
 * 进入一个入口：app 域唯一的「打开」入口。
 *
 * 载荷类型由 `meta.json` 声明决定，写错形状即编译报错；允许省略 ——
 * 地址可以没有载荷，此时浮层自行降级为不打开
 */
function enter<E extends AppEntryId>(entry: E, payload?: AppEntryPayload[E]): void {
  nav.go(action(entry, payload));
}

/** 离开一个入口：地址回退到父级，app 域唯一的「关闭」入口 */
function leave(entry: AppEntryId): void {
  // 只在自己确实是当前地址目标时才回退。否则别处已在导航：用户点侧边栏切卡片，
  // 导航先落地，浮层的 clickOutside 随后触发，再推一次会把刚切过去的卡片拽回来
  if (nav.currentRoute() !== path(entry)) {
    return;
  }
  const parent = APP_ENTRY_PARENT[entry];
  if (parent !== null) {
    nav.go(action(parent));
  }
}

export const appNavigate = Object.freeze({path, action, enter, leave});
