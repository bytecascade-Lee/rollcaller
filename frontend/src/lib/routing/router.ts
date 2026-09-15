/**
 * 窗口路由：**两个入口、一个处理函数**。
 *
 * 与 `HelpAddress` 的"三入口一个出口"同构——应用代码构造的 `Action` 与外部改动的
 * hash，最终都只走到一处 `handle`。所以不存在"两套机制各自为政"：这是把三个并存
 * 的跳转（card layout / popupController / 帮助文档导航）收成一个"门"的落点。
 *
 * 它只做三件事，且刻意只做这三件：
 *
 * 1. **规范化**：非法地址直接忽略（降级，不报错、不半应用）；
 * 2. **判归属**：只看 `route` 第一段是不是本窗口名。是 → `handle`，不是 → `forward`；
 * 3. **兜外**：`hashchange` 只兜"不是我们写进去的"那种改动（原生 `<a href="#/...">`、
 *    浏览器前进后退）。自己写的 hash 走 `replaceState`，本就不触发 `hashchange`，
 *    因此天然不会重复处理。
 *
 * 不在这里做的事（都是刻意排除的）：
 *
 * - **不写 hash**。地址的对外投影由窗口自己的 handler 收尾（就是 `replaceHash` 那一行），
 *   因为只有窗口知道"我现在的地址是什么"；
 * - **不做幂等去重**。重放同一个地址该是"重新定位"还是"重新应用初始状态"，只有 owner
 *   知道（帮助窗口是前者，页面弹窗是后者）。放在这里会替 owner 做错决定；
 * - **不认识 plane / overlay / doc / page**。槽位怎么分是窗口内部的事。
 */
import {
  canonicalAction,
  canonicalRoute,
  parseRoute,
  type Action,
  type Route,
} from "$routing/route";

export type Router = {
  /** 应用代码的唯一入口（按钮、搜索、跨窗口投递落地都走它） */
  go(action: Action): void;
  /**
   * 挂载：登记 hashchange 兜底，然后决定首帧落点。
   *
   * - `fallback`：当前 hash 不是本窗口的合法地址时用它（各窗口的默认落点）；
   * - `initial`：显式指定的首个地址，**优先于 hash**。冷启动投递的待取载荷就走这里
   *   （此时 hash 还是空的，但载荷已经躺在后端等着了）。
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
  const { host, handle, forward } = options;
  const self = host.toLowerCase();

  const go = (action: Action) => {
    const normalized = canonicalAction(action);
    if (!normalized) return;
    const parsed = parseRoute(normalized.route);
    if (!parsed) return;
    if (parsed.host !== self) {
      void forward?.(normalized);
      return;
    }
    void handle(normalized);
  };

  const start = (fallback: Action, initial?: Action): (() => void) => {
    const onHashChange = () => {
      const route: Route | null = canonicalRoute(location.hash);
      if (route) go({ route });
    };
    window.addEventListener("hashchange", onHashChange);

    if (initial) {
      go(initial);
    } else {
      const fromHash = canonicalRoute(location.hash);
      const parsed = fromHash === null ? null : parseRoute(fromHash);
      go(parsed && parsed.host === self ? { route: fromHash as Route } : fallback);
    }

    return () => window.removeEventListener("hashchange", onHashChange);
  };

  return { go, start };
}
