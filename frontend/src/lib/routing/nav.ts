/**
 * 当前窗口的导航单例
 *
 * 窗口的根组件（`App.svelte` / `Help.svelte`）用 `install()` 交出"归本窗口的地址落到哪里"，
 * 之后窗口里**任何组件、store、service** 都能用 `nav.go(action)` 发起跳转，不必层层传 props。
 *
 * 一次跳转的去处只取决于地址的 host：
 *
 * ```
 * nav.go(action)
 *   ├ host 是本窗口 → apply(action, parsed)   同窗口：直接落地
 *   └ 否则          → forward(action)         交后端投递给目标窗口
 * ```
 *
 * 未安装时静默忽略（窗口还没起来就有人调导航，不该炸）。
 */
import {type Action, canonicalRoute, hostOf, isRouteHref, type ParsedAction, parseAction, type Route,} from "$routing/address";

export type NavHandlers = {
  /** 本窗口名（= 地址的 host），比较时大小写不敏感 */
  host: string;
  /** 归本窗口的地址落到哪里；`parsed` 是已切好的地址，落地不必再切一次 */
  apply: (action: Action, parsed: ParsedAction) => void;
  /** 不归本窗口的地址交给谁（跨窗口投递；缺省则静默忽略） */
  forward?: (action: Action) => void;
};

let handlers: NavHandlers | null = null;

const selfHost = () => handlers?.host.toLowerCase() ?? null;

/** 应用代码的唯一入口：规范化 → 判归属 → 落地或转发 */
function go(action: Action): void {
  const parsed = parseAction(action);
  if (!parsed) {
    return;
  }
  const target: Action = {route: parsed.route, payload: parsed.payload};
  if (parsed.host !== selfHost()) {
    handlers?.forward?.(target);
    return;
  }
  handlers?.apply(target, parsed);
}

/**
 * 挂载：登记 `hashchange` 兜底，然后决定首帧落点。
 *
 * - `initial`：显式指定的首个地址（跨窗口投递 / 冷启动载荷），**优先于 hash**
 * - 否则 hash 若是本窗口的地址就用它，再否则 `fallback`
 *
 * 返回卸载函数。
 */
function start(fallback: Action, initial?: Action): () => void {
  const onHashChange = () => {
    const route = canonicalRoute(location.hash);
    if (route) {
      go({route});
    }
  };
  window.addEventListener("hashchange", onHashChange);

  if (initial) {
    go(initial);
  } else {
    const route = canonicalRoute(location.hash);
    go(route && hostOf(route) === selfHost() ? {route} : fallback);
  }

  return () => window.removeEventListener("hashchange", onHashChange);
}

export const nav = {
  /** 由窗口根组件安装，重复安装以最后一次为准 */
  install(next: NavHandlers): void {
    handlers = next;
  },

  go,

  start,

  /**
   * 当前地址（读 `location.hash` 并规范化）；不是应用内地址时为 `null`。
   *
   * 它是**只读的现状**，供"要不要再推一次导航"这类判断使用：例如关闭某个浮层时，
   * 得先确认自己仍是当前地址的目标——否则说明别处已经在导航，再推一次会把地址拽回去。
   */
  currentRoute(): Route | null {
    return canonicalRoute(location.hash);
  },

  /**
   * 只同步 `location.hash`，**不触发导航**——落地的真相在各窗口自己的 handler 上，hash 只是它的对外投影。
   *
   * 用 `replaceState` 而非赋值 `location.hash`：后者会产生浏览器历史条目，与各窗口自有的
   * 前进/后退栈冲突；且 `replaceState` **不触发 hashchange**，天然不会把"自己写的地址"再处理一遍。
   */
  replaceHash(route: Route): void {
    if (!isRouteHref(route)) {
      return
    }
    history.replaceState(null, "", route);
  },
};
