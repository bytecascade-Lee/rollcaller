import {APP_ENTRY_PARENT, type AppEntryId} from "$routing/app.entries.generated";

/** 入口名全集（运行时；`APP_ENTRY_PARENT` 的键投影）。用 Set 而非 `in`：后者会把 `toString` 之类当命中 */
const APP_ENTRY_NAMES = new Set<string>(Object.keys(APP_ENTRY_PARENT));

/** 某入口的直接子入口（由 `parentId` 推出）。页面用 `EntryRegistry<ChildOf<P>>` 把别页的入口名挡在编译期 */
export type ChildOf<P extends AppEntryId> = {
  [K in AppEntryId]: (typeof APP_ENTRY_PARENT)[K] extends P ? K : never
}[AppEntryId];

/** 自根到该入口的链（含自身）。`parentId` 无环由生成器保证，故这里不必防环 */
export function chainOf(entry: AppEntryId): AppEntryId[] {
  const chain: AppEntryId[] = [];
  let cur: AppEntryId | null = entry;
  while (cur !== null) {
    chain.unshift(cur);
    cur = APP_ENTRY_PARENT[cur];
  }
  return chain;
}

/** 由地址段解析整条链。严格解析：上游 `parseRoute` 刻意宽容 */
export function resolveAppChain(segments: readonly string[]): AppEntryId[] | null {
  const name = segments[segments.length - 1];
  if (!name || !APP_ENTRY_NAMES.has(name)) {
    return null;
  }
  const chain = chainOf(name as AppEntryId);
  if (chain.length !== segments.length) {
    return null;
  }
  return chain.every((id, index) => id === segments[index]) ? chain : null;
}

/** 只要末尾那个入口 */
export function resolveAppEntry(segments: readonly string[]): AppEntryId | null {
  return resolveAppChain(segments)?.at(-1) ?? null;
}
