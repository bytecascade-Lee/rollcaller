/** 帮助窗口的载荷：目前只有搜索框预填 */

export type HelpPayload = {search?: string};

/**
 * 从地址带来的载荷里取搜索词。
 *
 * 载荷跨窗口时是任意 JSON，故宽进窄出：认不出就当作「没要求」，退化为纯导航
 */
export function readSearch(payload: unknown): string | null {
  if (typeof payload !== "object" || payload === null) {
    return null;
  }
  const {search} = payload as HelpPayload;
  return typeof search === "string" && search.trim() !== "" ? search : null;
}
