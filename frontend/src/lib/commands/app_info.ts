import {invoke} from "@tauri-apps/api/core";
import {AppInfo} from "$types";

export async function app_info() {
  return await invoke<AppInfo>("app_info");
}
