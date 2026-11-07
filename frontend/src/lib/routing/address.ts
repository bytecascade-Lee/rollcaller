/** 应用内路由：`#/<host>/<seg1>/<seg2>/...`，分为 host 与 segments */

/** 处理者名 = 窗口名 */
export type Host = string;

/** 身份 = 路径 */
export type Route = `#/${string}`;

/** 导航载荷，要求可进行 JSON 序列化与反序列化 */
export type Payload = unknown;

/** 应用代码的完整表达：身份 + 载荷 */
export type Action = {route: Route; payload?: Payload};

/** 应用内路由前缀 */
export const ROUTE_PREFIX = "#/";

/** 已分段的切分结果 */
export type ParsedRoute = {host: Host; segments: string[]};

/** 规范化后的完整 Action：地址只剩一种写法，段已经切好，调用方不必再切一次 */
export type ParsedAction = {route: Route; host: Host; segments: string[]; payload?: Payload};

/**
 * 切分地址。不校验段语法，host 大小写不敏感，丢掉空段。
 *
 * 非应用内路由（外部链接、同文档锚点、绝对路径）返回 null
 */
export function parseRoute(raw: string): ParsedRoute | null {
  if (!raw) {
    return null;
  }
  const hash = raw.startsWith("#") ? raw.slice(1) : raw;
  if (!hash.startsWith("/")) {
    return null;
  }
  const segments = hash
    .split("/")
    .slice(1)
    .filter((segment) => segment !== "");
  const [rawHost, ...rest] = segments;
  if (!rawHost) {
    return null;
  }
  return {
    host: rawHost.toLowerCase(),
    segments: rest,
  };
}

/** 组装地址 */
export function routeOf(host: Host, segments: readonly string[] = []): Route {
  const tail = segments.filter((segment) => segment !== "").join("/");
  const normalizedHost = host.toLowerCase();
  return `${ROUTE_PREFIX}${normalizedHost}${tail ? `/${tail}` : ""}` as Route;
}

/** 规范化任意形式的地址（切分后重组）；非法返回 null */
export function canonicalRoute(raw: string): Route | null {
  const parsed = parseRoute(raw);
  return parsed ? routeOf(parsed.host, parsed.segments) : null;
}

/**
 * 规范化 Action 并一次切分到位；route 非法时返回 null。
 *
 * 导航链路上唯一的切分点：`go` 判归属、落地构链都用它的结果
 */
export function parseAction(action: Action): ParsedAction | null {
  const parsed = parseRoute(action.route);
  if (!parsed) {
    return null;
  }
  return {
    route: routeOf(parsed.host, parsed.segments),
    host: parsed.host,
    segments: parsed.segments,
    payload: action.payload,
  };
}

/** 是否应用内路由 */
export function isRouteHref(href: string): boolean {
  return href.startsWith(ROUTE_PREFIX);
}

/** 取 host */
export function hostOf(raw: string): Host | null {
  return parseRoute(raw)?.host ?? null;
}
