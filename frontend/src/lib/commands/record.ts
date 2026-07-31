import {invoke} from "@tauri-apps/api/core";
import {RollcallRecord} from "$types/RollcallRecord";
import {Record} from "$types/Record";


export async function list() {
  return await invoke<RollcallRecord[]>("record_list");
}

export async function create(record: Record) {
  return await invoke<RollcallRecord>("record_single_create", {
    record: record
  })
}

export async function update(ids: bigint[], attendance_status: number, remark: string) {
  return await invoke<RollcallRecord[]>("record_batch_update", {
    ids: ids,
    attendanceStatus: attendance_status,
    remark: remark
  })
}

export async function update_attendance_status(ids: bigint[], attendance_status: number) {
  return await invoke<RollcallRecord[]>("record_batch_update_attendance_status", {
    ids: ids,
    attendanceStatus: attendance_status
  })
}

export async function update_remark(ids: bigint[], remark: string) {
  return await invoke<RollcallRecord[]>("record_batch_update_remark", {
    ids: ids,
    remark: remark
  })
}
