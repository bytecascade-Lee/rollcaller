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


const app = (...segments: string[]): Action => ({route: routeOf("app", segments)});

/** 主窗口落点 */
export const AppEntries = {
  rollcall: app("rollcall"),
  students: app("students"),
  records: app("records"),

  studentEdit: app("students", "edit"),

  recordEdit: app("records", "edit"),
} as const satisfies Record<string, Action>;
