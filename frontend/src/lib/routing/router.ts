/**
 * 窗口路由
 *
 * ## 职责
 *
 * 1. **规范化**：忽略非法地址
 * 2. **判归属**：检查 `route` 第一段是否为本窗口名
 *    - 是 -> `handle`
 *    - 否 -> `forward`
 */
import {type Action, canonicalAction, canonicalRoute, parseRoute, type Route,} from "$routing/address";

export type Router = {
  /** 应用代码的唯一入口 */
  go(action: Action): void;
  /**
   * 挂载：登记 hashchange 兜底，然后决定首帧落点。
   *
   * - `fallback`：当前 hash 不是本窗口的合法地址时
   * - `initial`：显式指定的首个地址，**优先于 hash**
   *
   * 返回卸载函数。
   */
  start(fallback: Action, initial?: Action): () => void;
};

export function createRouter(options: {
  /** 本窗口名（= host） */
  host: string;
  /** 归本窗口的地址落到哪里 */
  handle: (action: Action) => void | Promise<void>;
  /** 不归本窗口的地址交给谁（跨窗口投递；缺省则静默忽略） */
  forward?: (action: Action) => void | Promise<void>;
}): Router {
  const {host, handle, forward} = options;
  const self = host.toLowerCase();

  const go = (action: Action) => {
    const normalized = canonicalAction(action);
    if (!normalized) {
      return
    }
    const parsed = parseRoute(normalized.route);
    if (!parsed) {
      return
    }
    if (parsed.host !== self) {
      void forward?.(normalized);
      return;
    }
    void handle(normalized);
  };

  const start = (fallback: Action, initial?: Action): (() => void) => {
    const onHashChange = () => {
      const route: Route | null = canonicalRoute(location.hash);
      if (route) {
        go({route})
      }
    };
    window.addEventListener("hashchange", onHashChange);

    if (initial) {
      go(initial);
    } else {
      const fromHash = canonicalRoute(location.hash);
      const parsed = fromHash === null ? null : parseRoute(fromHash);
      go(parsed && parsed.host === self ? {route: fromHash as Route} : fallback);
    }

    return () => window.removeEventListener("hashchange", onHashChange);
  };

  return {
    go,
    start
  };
}
