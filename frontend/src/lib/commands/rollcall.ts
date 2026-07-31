import {invoke} from "@tauri-apps/api/core";

export async function pick(ids: bigint[]) {
  return await invoke<bigint>("pick", {
    ids: ids
  })
}
