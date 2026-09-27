// 本文件由 scripts/generate_routing_entries.py 生成，请勿手改。
// 源：frontend/src/lib/resources/app/meta.json

/** 本表的 host（= 窗口名）：地址 `#/<host>/...` 的第一段 */
export const APP_HOST = "app";

/** 全部入口名。表内唯一，也是地址中除 host 外的合法段 */
export type AppEntryId =
  | "rollcall"
  | "student-management"
  | "record-history"
  | "create-student"
  | "import-student"
  | "edit-student"
  | "export-student"
  | "delete-student"
  | "edit-record"
  | "export-record";

/** 入口 → 归属。`parentId` 为 null 者是树的根（卡片） */
export const APP_ENTRIES = {
  "rollcall": {"parentId": null},
  "student-management": {"parentId": null},
  "record-history": {"parentId": null},
  "create-student": {"parentId": "student-management"},
  "import-student": {"parentId": "student-management"},
  "edit-student": {"parentId": "student-management"},
  "export-student": {"parentId": "student-management"},
  "delete-student": {"parentId": "student-management"},
  "edit-record": {"parentId": "record-history"},
  "export-record": {"parentId": "record-history"}
} as const satisfies Record<AppEntryId, {parentId: AppEntryId | null}>;

/** 全部入口名（运行时投影，顺序即 meta 的书写顺序） */
export const APP_ENTRY_IDS = [
  "rollcall",
  "student-management",
  "record-history",
  "create-student",
  "import-student",
  "edit-student",
  "export-student",
  "delete-student",
  "edit-record",
  "export-record"
] as const satisfies readonly AppEntryId[];
