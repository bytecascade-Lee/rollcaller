import {type Action, routeOf} from "$routing/address";

/** 帮助窗口落点 */
const help = (doc: string, anchor?: string): Action => ({
  route: routeOf("help", anchor ? [doc, anchor] : [doc]),
});

/** 忽略 `meta.json` 中的父节点 */
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
 * app 窗口的卡片
 */
export const AppCards = {
  rollcall: "rollcall",
  studentManagement: "student-management",
  recordHistory: "record-history",
} as const;

export type AppCardKey = keyof typeof AppCards;

/** 卡片 id（地址第一段） */
export type AppCard = (typeof AppCards)[AppCardKey];

/** 卡片 id 列表（运行时白名单） */
export const AppCardIds: readonly AppCard[] = Object.values(AppCards);

/** `#/app/<卡片>` */
const card = (id: AppCard): Action => ({route: routeOf("app", [id])});

/** `#/app/<卡片>/<叠加层>` */
const overlay = (target: AppCard, name: string): Action => ({route: routeOf("app", [target, name])});

/** 卡片落点：与 `AppCards` 的 key 一一对应 */
const cardEntries = {
  rollcall: card(AppCards.rollcall),
  studentManagement: card(AppCards.studentManagement),
  recordHistory: card(AppCards.recordHistory),
} satisfies Record<AppCardKey, Action>;

/** 主窗口落点 */
export const AppEntries = {
  ...cardEntries,

  studentEdit: overlay(AppCards.studentManagement, "edit"),
} as const satisfies Record<string, Action>;

/**
 * 各落点的实现方（组件名）：与 `app/meta.json` 的 `owner` 一一对应
 *
 * `null` owner 表示"声明了名字但还没实现"，见 `overlayNames.ts` 的 `restore`
 */
export const AppOwners = [
  "RollcallPage",
  "StudentManagementPage",
  "RecordHistoryPage",
  "SingleCreateStudent",
  "EditStudent",
  "DeleteStudents",
  "ImportStudents",
  "ExportStudents",
  "EditRecord",
  "ExportRecords",
] as const;

/** `studentEdit` 的载荷 */
export type StudentEditPayload = { ids: number[] };

/** 载荷字段名：与上面类型双向绑定（少一个/多一个都编译报错），供校验器比对 `app/meta.json` */
export const StudentEditFields = Object.keys(
  {ids: true} satisfies Record<keyof StudentEditPayload, true>,
);
