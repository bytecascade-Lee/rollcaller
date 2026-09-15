/**
 * 应用内路由：`#/<host>/<seg1>/<seg2>/...`
 *
 * 地址只分两半，这是整套设计的地基：
 *
 * - **host**（第一段）= 处理者名，恰好等于窗口名（`app` / `help`）。全局只认这一件
 *   事——够它判断"这归不归我、不归我该交给谁"，于是后端不需要"域 → 窗口"映射表；
 * - **segments**（其余段）= 不透明路径。语义由 owner 声明（位置即语法），全局不解释，
 *   这正是"路由薄、域厚"。
 *
 * 只有大小写一条全局规则：**host 归一为小写**（后端窗口名全小写，`get_webview_window`
 * 的字面量也是小写）。**段的大小写不归一**——文档 id 里有 `README.md` 这种带点带大写的
 * 名字，全局替它定规则会跟文档命名打架；段的合法性是域自己的事，由构建期校验器管。
 *
 * 地址**不携带数据**：一切"带什么"走 `Payload`。两者分开的理由是它们的能力不同——
 * `route` 是字符串，能写进 markdown 的 href、能当 IPC 参数、能直接比较；
 * `payload` 是数据，只能交给目标 owner 解释，且必须可 JSON 序列化、不可执行。
 */

/** 处理者名 = 窗口名 */
export type Host = string;

/** 身份：只写路径。字符串形式是 `#/<host>[/<seg>...]`，可直接写进 href */
export type Route = string;

/** 数据：只交给目标 owner 解释。必须可 JSON 序列化、不可执行（禁止函数/回调） */
export type Payload = Record<string, unknown>;

/** 应用代码的完整表达：身份 + 载荷 */
export type Action = { route: Route; payload?: Payload };

/**
 * 应用内路由前缀。
 * `#/` 才是"应用内路由"与"本文档原生锚点"（`#v0-8-1`）的判别位——靠的是那个斜杠，
 * 不是大小写，所以域名统一小写并不损失可辨识度。
 */
export const ROUTE_PREFIX = "#/";

/** 切分结果：已分段，但段语义未解释 */
export type ParsedRoute = { host: Host; segments: string[] };

/**
 * 切分地址。
 *
 * 宽容：不校验段的语法（那是域的事），host 大小写不敏感，丢掉空段
 * （`#/help//x` ≡ `#/help/x`——空段白拿一个规范化）。
 * 非应用内路由（外部链接、同文档锚点、绝对路径）一律返回 null。
 */
export function parseRoute(raw: string): ParsedRoute | null {
  if (!raw) return null;
  const hash = raw.startsWith("#") ? raw.slice(1) : raw;
  if (!hash.startsWith("/")) return null;
  const segments = hash.split("/").slice(1).filter((segment) => segment !== "");
  const [rawHost, ...rest] = segments;
  if (!rawHost) return null;
  return { host: rawHost.toLowerCase(), segments: rest };
}

/** 组装地址（canonical 形式：host 小写、段原样、无空段、无尾斜杠） */
export function routeOf(host: Host, segments: readonly string[] = []): Route {
  const tail = segments.filter((segment) => segment !== "").join("/");
  const normalizedHost = host.toLowerCase();
  return tail ? `${ROUTE_PREFIX}${normalizedHost}/${tail}` : `${ROUTE_PREFIX}${normalizedHost}`;
}

/** 规范化任意形式的地址（切分后重组）；非法返回 null */
export function canonicalRoute(raw: string): Route | null {
  const parsed = parseRoute(raw);
  return parsed ? routeOf(parsed.host, parsed.segments) : null;
}

/**
 * 规范化一个 Action。route 非法时返回 null，调用方应**降级为忽略**——
 * 不报错、不半应用（与锚点缺失回文档顶部同一种降级思路）。
 */
export function canonicalAction(action: Action): Action | null {
  const route = canonicalRoute(action.route);
  return route ? { route, payload: action.payload } : null;
}

/** 是否应用内路由：渲染期判 href 用，不解析、不抛错 */
export function isRouteHref(href: string): boolean {
  return href.startsWith(ROUTE_PREFIX);
}

/** 取 host（等价于 `parseRoute(raw)?.host`，给只需判归属的调用点省一次解构） */
export function hostOf(raw: string): Host | null {
  return parseRoute(raw)?.host ?? null;
}

/**
 * 只同步 `location.hash`，**不触发导航**——导航的落地由窗口自己的 handler 负责，
 * hash 只是它的对外投影。
 *
 * 用 `replaceState` 而非赋值 `location.hash`：后者会产生浏览器历史条目，
 * 与各窗口自有的前进/后退栈（帮助窗口标题栏那两个箭头）打架；而且
 * `replaceState` **不触发 hashchange**，天然不会把"自己写的地址"再处理一遍。
 */
export function replaceHash(route: Route): void {
  if (!isRouteHref(route)) return;
  history.replaceState(null, "", route);
}
