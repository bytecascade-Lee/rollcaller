import {type Action, routeOf} from "$routing/address";
import {type AppEntryId, APP_ENTRIES, APP_HOST} from "$routing/entries.generated";

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

/**
 * 某个入口的**直接子入口**（由声明的 `parentId` 推出）。
 *
 * 页面只应打开自己子树里的入口，于是页面写 `createOverlayRegistry<ChildOf<"student-management">>()`，
 * 在该页 `open("edit-record")` 会在编译期被拦下 —— 这正是旧 `StudentOverlayName` 保住的性质，
 * 而成员全部来自 `meta.json`，代码里没有第二份名字表。
 */
export type ChildOf<P extends AppEntryId> = {
  [K in AppEntryId]: (typeof APP_ENTRIES)[K]["parentId"] extends P ? K : never
}[AppEntryId];

/** 树的根入口（卡片）：`parentId` 为 null 者 */
export type RootEntryId = {
  [K in AppEntryId]: (typeof APP_ENTRIES)[K]["parentId"] extends null ? K : never
}[AppEntryId];

/**
 * app 窗口的卡片
 *
 * 值手写，但受 `RootEntryId` 约束：`meta.json` 里改了卡片名，这里立刻编译报错，不可能静默漂移。
 */
export const AppCards = {
  rollcall: "rollcall",
  studentManagement: "student-management",
  recordHistory: "record-history",
} as const satisfies Record<string, RootEntryId>;

export type AppCardKey = keyof typeof AppCards;

/** 卡片 id */
export type AppCard = (typeof AppCards)[AppCardKey];

/** 卡片 id 列表（运行时白名单） */
export const AppCardIds: readonly AppCard[] = Object.values(AppCards);

/** `#/<host>/<卡片>` */
const card = (id: AppCard): Action => ({route: routeOf(APP_HOST, [id])});

/** `#/<host>/<卡片>/<入口名>` */
const overlay = (parent: AppCard, name: AppEntryId): Action =>
  ({route: routeOf(APP_HOST, [parent, name])});

/** 卡片落点：与 `AppCards` 的 key 一一对应 */
const cardEntries = {
  rollcall: card(AppCards.rollcall),
  studentManagement: card(AppCards.studentManagement),
  recordHistory: card(AppCards.recordHistory),
} satisfies Record<AppCardKey, Action>;

/** 主窗口落点 */
export const AppEntries = {
  ...cardEntries,

  studentEdit: overlay(AppCards.studentManagement, "edit-student"),
} as const satisfies Record<string, Action>;

/** `studentEdit` 的载荷 */
export type StudentEditPayload = { ids: number[] };

/** 载荷字段名：与上面类型双向绑定（少一个/多一个都编译报错） */
export const StudentEditFields = Object.keys(
  {ids: true} satisfies Record<keyof StudentEditPayload, true>,
);
