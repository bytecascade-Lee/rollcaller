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
 *   ├- host 是本窗口 -> apply(action, parsed)   同窗口则直接落地
 *   └- 否则          -> forward(action)         交后端投递给目标窗口
 * ```
 *
 * 未安装时静默忽略，保留 warn
 */
import {
  type Action,
  canonicalRoute,
  hostOf,
  isRouteHref,
  parseAction,
  type ParsedAction,
  type Route,
} from "$routing/address";
import type {WindowKind} from "$types";

export type NavHandlers = {
  /** 本窗口名（= 地址的 host）。闭集枚举：写错即编译错误 */
  host: WindowKind;
  /** 归本窗口的地址落到哪里；`parsed` 是已切好的地址，落地不必再切一次 */
  apply: (action: Action, parsed: ParsedAction) => void;
  /** 不归本窗口的地址交给谁（跨窗口投递；缺省则静默忽略） */
  forward?: (action: Action) => void;
};

let handlers: NavHandlers | null = null;

/**
 * 判归属 → 落地或转发 —— **唯一的分发点**。
 *
 * 入参是**已切好**的地址：解析只发生在 `go` 与 `start` 两处，落地侧一律不再碰字符串。
 */
function dispatch(parsed: ParsedAction): void {
  const target: Action = {route: parsed.route, payload: parsed.payload};
  if (handlers === null) {
    console.warn(`[routing] 导航尚未安装，已忽略：${parsed.route}`);
    return;
  }
  if (parsed.host !== handlers.host) {
    handlers.forward?.(target);
    return;
  }
  handlers.apply(target, parsed);
}

/** 应用代码的唯一入口：规范化 → 判归属 → 落地或转发 */
function go(action: Action): void {
  const parsed = parseAction(action);
  if (!parsed) {
    console.warn(`[routing] 不是应用内地址，已忽略：${action.route}`);
    return;
  }
  dispatch(parsed);
}

/** 当前 hash 若是本窗口的地址就取它，否则 null */
function hashAction(): Action | null {
  const route = canonicalRoute(location.hash);
  if (route === null || handlers === null) {
    return null;
  }
  return hostOf(route) === handlers.host ? {route} : null;
}

/**
 * 挂载：登记 `hashchange` 兜底，然后决定首帧落点。
 *
 * 首帧来源的优先级：**合法的 `initial`**（跨窗口投递 / 冷启动载荷）→ 本窗口的 hash → `fallback`。
 * `initial` 非法时必须继续往下兜 —— 否则整条首帧路径被吞掉，窗口会停在空链上（连卡片都不激活）。
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

  const first = (initial && parseAction(initial)) || parseAction(hashAction() ?? fallback);
  if (first) {
    dispatch(first);
  } else {
    console.warn("[routing] 首帧地址与 fallback 都非法，未落地");
  }

  return () => window.removeEventListener("hashchange", onHashChange);
}

export const nav = {
  /**
   * 由窗口根组件安装，返回卸载函数。
   *
   * 重复安装以最后一次为准，但会警告：热重载与测试都要显式 dispose，否则旧 handler 会
   * 一直挂在上面（`handlers` 是模块级单例，没有别的回收途径）。
   */
  install(next: NavHandlers): () => void {
    if (handlers !== null) {
      console.warn("[routing] nav.install 被重复调用：以最后一次为准（上一份未 dispose）");
    }
    handlers = next;
    return () => {
      if (handlers === next) {
        handlers = null;
      }
    };
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
