/**
 * 应用内路由：`#/<host>/<seg1>/<seg2>/...`
 *
 * 地址分为 **host** 和 **segments**
 */

/** 处理者名 = 窗口名 */
export type Host = string;

/** 身份 = 路径 */
export type Route = string;

/** 数据只交给目标 owner 解释。必须可 JSON 序列化、不可执行 */
export type Payload = Record<string, unknown>;

/** 应用代码的完整表达：身份 + 载荷 */
export type Action = { route: Route; payload?: Payload };

/** 应用内路由前缀 */
export const ROUTE_PREFIX = "#/";

/** 已分段的切分结果 */
export type ParsedRoute = { host: Host; segments: string[] };

/**
 * 切分地址
 *
 * ## 宽容处理
 * - 不校验段的语法
 * - host 大小写不敏感
 * - 丢掉空段
 *
 * 对于非应用内路由（外部链接、同文档锚点、绝对路径），返回 null
 */
export function parseRoute(raw: string): ParsedRoute | null {
  if (!raw) {
    return null
  }
  const hash = raw.startsWith("#") ? raw.slice(1) : raw;
  if (!hash.startsWith("/")) {
    return null
  }
  const segments = hash.split("/").slice(1).filter((segment) => segment !== "");
  const [rawHost, ...rest] = segments;
  if (!rawHost) {
    return null
  }
  return {
    host: rawHost.toLowerCase(),
    segments: rest
  };
}

/** 组装地址 */
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
 * 规范化一个 Action。route 非法时返回 null
 */
export function canonicalAction(action: Action): Action | null {
  const route = canonicalRoute(action.route);
  return route ? {route, payload: action.payload} : null;
}

/** 是否应用内路由 */
export function isRouteHref(href: string): boolean {
  return href.startsWith(ROUTE_PREFIX);
}

/** 取 host */
export function hostOf(raw: string): Host | null {
  return parseRoute(raw)?.host ?? null;
}

