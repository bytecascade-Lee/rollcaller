import {invoke} from "@tauri-apps/api/core";
import type {RollcallRecord} from "$lib/types/RollcallRecord";

export let records = $state<RollcallRecord[]>([]);
export let boundaryPoint = 0n;
export let isLoading = $state(false);

export async function load() {
  isLoading = true;
  try {
    records = await invoke<RollcallRecord[]>("list_all_records");
  } finally {
    isLoading = false;
    for (let record of records) {
      if (boundaryPoint < record.id) {
        boundaryPoint = record.id;
      }
    }
  }
}

export function upsert(record: RollcallRecord) {
  const index = records.findIndex((s) => s.id === record.id);
  if (index >= 0) {
    records = [...records.slice(0, index), record, ...records.slice(index + 1)];
  } else {
    records = [...records, record];
  }
}

export function remove(ids: bigint[]) {
  records = records.filter((s) => !ids.includes(s.id));
}
