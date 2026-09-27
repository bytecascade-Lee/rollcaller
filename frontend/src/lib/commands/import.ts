import {invoke} from "@tauri-apps/api/core";
import {SheetPreview, StudentBatchCreateResult} from "$types";

/** 预览文件：一次读回全部工作表（每张表自带前若干行与行列数） */
export async function preview(filePath: string) {
  return await invoke<SheetPreview[]>("preview_excel", {
    filePath: filePath
  })
}

export async function load(
  filePath: string,
  sheetName: string,
  headerRows: number,
  studentNoColumn: number,
  nameColumn: number,
  decisions: Record<string, boolean>
) {
  return await invoke<StudentBatchCreateResult>("import_excel", {
    filePath: filePath,
    sheetName: sheetName,
    headerRows: headerRows,
    studentNoColumn: studentNoColumn,
    nameColumn: nameColumn,
    decisions: decisions
  })
}
