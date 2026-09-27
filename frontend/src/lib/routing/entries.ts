import {type Action, type Payload, routeOf} from "$routing/address";
import {type AppEntryId, APP_ENTRY_PARENT, APP_HOST} from "$routing/entries.generated";
import {WindowRouter} from "$routing/windowRouter";

/** 帮助窗口落点 */
const help = (doc: string, anchor?: string): Action => ({
  route: routeOf("help", anchor ? [doc, anchor] : [doc]),
});

/** 帮助窗口的策展式深链：`help/meta.json` 只有文档节点，没有这些"文档 + 小节"的组合 */
export const HelpEntries = {
  overview: help("overview"),
  quickStart: help("quick-start", "install-app"),

  singleRollcall: help("single-rollcall", "operation-steps"),
  autoFinish: help("auto-finish", "operation-steps"),
  manualPause: help("manual-pause", "operation-steps"),
  ttsModes: help("tts", "modes"),
  ttsPlayback: help("tts", "playback-behavior"),

  addStudent: help("add-student", "operation-steps"),
  importStudents: help("batch-import", "import-steps"),
  editStudent: help("edit-student", "operation-steps"),
  deleteStudent: help("delete-student", "operation-steps"),
  exportStudent: help("export-student", "operation-steps"),

  editRecord: help("edit-record", "operation-steps"),
  exportRecord: help("export-record", "operation-steps"),

  appWindow: help("app-window", "sidebar"),
  helpWindow: help("help-window", "tree-nav"),
} as const satisfies Record<string, Action>;

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

/** 入口的地址：`#/<host>/<祖先...>/<入口>`。**地址由链推出，不另存一份常量** */
export function appPath(entry: AppEntryId): string {
  return routeOf(APP_HOST, chainOf(entry));
}

/** 入口 + 载荷 → 可投递的 Action（跨窗口时走它） */
export function appAction(entry: AppEntryId, payload?: Payload): Action {
  return {route: appPath(entry), payload};
}

/**
 * **进入**一个入口 —— app 域唯一的"打开"入口。
 *
 * 一切打开都走地址（哪怕目标就在本窗口）：这样"地址"与"实际显示"永远是同一件事，
 * 深链、刷新、重放、跨窗口四条路走的是同一条。
 */
export function appEnter(entry: AppEntryId, payload?: Payload): void {
  WindowRouter.nav(appAction(entry, payload));
}

/**
 * **离开**一个入口：地址回退到它的父级 —— app 域唯一的"关闭"入口。
 *
 * 关闭必须也是导航：否则地址会停在"已经关掉的位置"上（地址撒谎），重放同一地址会被
 * 转换的 diff 判定为无事可做而静默失效。
 *
 * 根入口没有父级，故是空操作（卡片本来也不可关）。
 */
export function appLeave(entry: AppEntryId): void {
  // 只在自己**确实是当前地址的目标**时才回退。否则说明别处已经在导航 —— 典型场景：
  // 浮层锚定在工具栏上，用户点侧边栏切卡片，侧边栏的导航先落地（冒泡顺序），随后
  // 浮层的 clickOutside 才触发；若这里再推一次，就会把刚切过去的卡片拽回来。
  if (WindowRouter.currentRoute() !== appPath(entry)) {
    return;
  }
  const parent = APP_ENTRY_PARENT[entry];
  if (parent !== null) {
    WindowRouter.nav(appAction(parent));
  }
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
