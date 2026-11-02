/**
 * 各页面可见的叠加层名字
 */
import type {ChildOf} from "$routing/app/entries";

/** 学生管理页 */
export type StudentOverlayName = ChildOf<"student-management">;

/** 历史记录页 */
export type RecordOverlayName = ChildOf<"record-history">;

/** 帮助窗口 */
export type HelpOverlayName = "search";

/** 帮助窗口浮层的载荷（对应 app 的 `AppEntryPayload`）。只有一个浮层，故手写而不生成 */
export type HelpOverlayPayload = { search: { query?: string } };

/** 帮助窗口浮层的句柄：调用点引用它，不写裸字符串 */
export const HelpOverlay = {search: "search"} as const satisfies Record<string, HelpOverlayName>;
