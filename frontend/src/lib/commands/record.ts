import {invoke} from "@tauri-apps/api/core";
import {RollcallRecord} from "$types/RollcallRecord";


export async function list() {
  return await invoke<RollcallRecord[]>("record_list");
}

export async function update(ids: bigint[], attendance_status: number, remark: string) {
  return await invoke<RollcallRecord[]>("record_batch_update", {
    ids: ids,
    attendance_status: attendance_status,
    remark: remark
  })
}
