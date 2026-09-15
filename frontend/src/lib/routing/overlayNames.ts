/**
 * 各页面的叠加层名字集合。
 *
 * 与旧的 `PopupName`（跨全应用的 12 个名字的单一命名空间）的区别是**按页面分组**：
 * 注册表归页面所有，名字只要**本页**唯一就够，页面拿到它之后能开的名字、弹窗能注册的
 * 名字都由同一个联合类型约束——写错编译期就报错，而不是靠命名约定加运行时静默失败。
 *
 * 最好的证据就是 `"edit"` / `"export"` 在两个页面里各有一个、彼此无关：单一全局命名
 * 空间下这不可能（所以旧名字才要拖上 `Student` / `Record` 前缀），收敛之后前缀就是纯噪音。
 *
 * 新增叠加层时在这里加名字。**只有需要被地址指到的**（可深链、可跨窗口投递）才配得上
 * 一条路由，见 `entries.ts` 的 `AppEntries`；纯过程（向导、确认、导出）有名字但没有地址。
 */

/** 学生管理页 */
export type StudentOverlayName = "create" | "edit" | "delete" | "import" | "export" | "restore";

/** 历史记录页 */
export type RecordOverlayName = "edit" | "export";

/** 帮助窗口（一个窗口一个页面，故直接按窗口命名） */
export type HelpOverlayName = "search";
