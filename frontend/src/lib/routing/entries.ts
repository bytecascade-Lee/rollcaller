import {APP_ENTRY_PARENT, type AppEntryId} from "$routing/entries.generated";

/** 入口名全集（运行时；`APP_ENTRY_PARENT` 的键投影）。用 Set 而非 `in`：后者会把 `toString` 之类当命中 */
const APP_ENTRY_NAMES = new Set<string>(Object.keys(APP_ENTRY_PARENT));

/**
 * 某个入口的**直接子入口**（由 `parentId` 推出）。
 *
 * 页面只应打开自己子树里的入口，于是页面写 `createEntryRegistry<ChildOf<"student-management">>()`，
 * 在该页 `show("edit-record")` 会在编译期被拦下。成员全部来自 `meta.json`，代码里没有第二份名字表。
 */
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

/**
 * 由地址段解析入口 —— **app 域唯一的地址解释者**。
 *
 * 严格（不给"猜"的余地）：最后一段必须是入口名，段数必须等于该入口的链长，
 * 且逐段与 `parentId` 链一致。任一不满足返回 `null`，调用方降级为"什么都不做"。
 */
export function resolveAppEntry(segments: readonly string[]): AppEntryId | null {
  const name = segments[segments.length - 1];
  if (!name || !APP_ENTRY_NAMES.has(name)) {
    return null;
  }
  const entry = name as AppEntryId;
  const chain = chainOf(entry);
  if (chain.length !== segments.length) {
    return null;
  }
  return chain.every((id, index) => id === segments[index]) ? entry : null;
}

/** `edit-student` / `delete-student` 共用的载荷：要操作哪些学生 */
export type StudentIdsPayload = { ids: number[] };

/** 载荷字段名：与上面类型双向绑定（少一个/多一个都编译报错） */
export const StudentIdsFields = Object.keys(
  {ids: true} satisfies Record<keyof StudentIdsPayload, true>,
);

/** 读载荷里的 ids：非法一律给空数组，调用方据此降级为"纯导航" */
export function readStudentIds(payload: unknown): number[] {
  const raw = (payload as StudentIdsPayload | undefined)?.ids;
  if (!Array.isArray(raw)) return [];
  return raw.filter((id): id is number => Number.isSafeInteger(id));
}
