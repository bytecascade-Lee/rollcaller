import {invoke} from "@tauri-apps/api/core";
import {AttendanceStatus} from "$types";

export async function list() {
  let statues = await invoke<AttendanceStatus[]>("attendance_status_list", {});
  return new Map(statues.map(status => [status.id, status]))
}
