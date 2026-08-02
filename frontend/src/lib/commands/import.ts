import {invoke} from "@tauri-apps/api/core";
import {ImportPreviewData, StudentBatchCreateResult} from "$types";

export async function preview(filePath: string) {
  return await invoke<ImportPreviewData>("preview_excel", {
    filePath: filePath
  })
}

export async function load(filePath: string, headerRows: number, columnMapping: Map<string, number>, decisions: Map<string, boolean>) {
  return await invoke<StudentBatchCreateResult>("import_excel", {
    filePath: filePath,
    headerRows: headerRows,
    columnMapping: columnMapping,
    decisions: decisions
  })
}
