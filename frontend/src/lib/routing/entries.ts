/**
 * 具名入口表：调用方写"我要『批量导入』的帮助"，而不是裸 id / 裸路径。
 *
 * 为什么要有这一层：裸字符串一旦散进 `.svelte`，改锚点或改路径就得全局搜，而且
 * 构建期校验器管不到组件里的字面量（这正是上一个注解体系腐烂的同一处伤口）。
 * 具名入口把"名字 → 地址"的对应收在一处，改地址只改这里。
 *
 * 帮助侧的表受构建期校验：`tests/help-address.test.mjs` 会逐个校验文档与锚点真实存在。
 *
 * 目前是手工维护的**最小集**，只收录应用代码真正会用到的落点。等入口多起来，
 * 帮助侧应由脚本从 `meta.json` + 文档目录生成——与导航树同源，这样"树"和"表"
 * 不可能各说各话。应用侧的落点则应该由路由表（routes.json）派生，那时这张表就是
 * 路由表的投影。
 */
import { routeOf, type Action } from "$routing/route";

/* ------------------------------------------------------------------ 帮助窗口 */

/** 帮助窗口的一个落点：文档 + 可选锚点，都是字符串，故没有载荷 */
const help = (doc: string, anchor?: string): Action => ({
  route: routeOf("help", anchor ? [doc, anchor] : [doc]),
});

/**
 * 只收**有内容**的文档：`meta.json` 里的纯父节点（`rollcall` / `students` / `records`
 * / `ui` / `about`）和 `multi-rollcall` 都没有对应的 .md 文件，落上去只会看到
 * "当前节点下暂无内容"，故不进表。
 */
export const HelpEntries = {
  /** 概览：帮助窗口没有指定落点时的默认文档 */
  overview: help("overview"),
  quickStart: help("quick-start", "install-app"),

  /** 点名 */
  singleRollcall: help("single-rollcall", "operation-steps"),
  autoFinish: help("auto-finish", "operation-steps"),
  manualPause: help("manual-pause", "operation-steps"),
  /** 语音播报：三态开关就在点名卡片上，这是"从文档跳到那个控件"的落点 */
  ttsModes: help("tts", "modes"),
  ttsPlayback: help("tts", "playback-behavior"),

  /** 学生管理 */
  addStudent: help("add-student", "operation-steps"),
  importStudents: help("batch-import", "import-steps"),
  editStudent: help("edit-student", "operation-steps"),
  deleteStudent: help("delete-student", "operation-steps"),
  exportStudent: help("export-student", "operation-steps"),

  /** 历史记录 */
  editRecord: help("edit-record", "operation-steps"),
  exportRecord: help("export-record", "operation-steps"),

  /** 界面说明 */
  appWindow: help("app-window", "sidebar"),
  helpWindow: help("help-window", "tree-nav"),
} as const satisfies Record<string, Action>;

/* ------------------------------------------------------------------ 主窗口 */

const app = (...segments: string[]): Action => ({ route: routeOf("app", segments) });

/**
 * 主窗口的落点。段结构是"plane 在前、overlay 在后"：
 * `#/app/<卡片>` 是平面视图，`#/app/<卡片>/<叠加层>` 是该卡片上的叠加层。
 *
 * 只有**带实体身份的界面态**才配得上一条地址（配载荷 `{ids: number[]}`）；
 * 纯过程（新建、删除确认、导入向导、导出）没有地址——它们的"位置"就是当前卡片，
 * 靠用户点按钮即可，不需要被外部指到。
 */
export const AppEntries = {
  /** 三张卡片 */
  rollcall: app("rollcall"),
  students: app("students"),
  records: app("records"),

  /** 学生管理：编辑指定学生（载荷 `{ids: number[]}`） */
  studentEdit: app("students", "edit"),

  /** 历史记录：批量修改指定记录（载荷 `{ids: number[]}`） */
  recordEdit: app("records", "edit"),
} as const satisfies Record<string, Action>;
