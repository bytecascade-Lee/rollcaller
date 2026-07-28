import {invoke} from "@tauri-apps/api/core";
import type {RollcallRecord} from "$lib/types/RollcallRecord";

export let records = $state<RollcallRecord[]>([]);
export let selected = $state<Set<bigint>>(new Set());
export let boundaryPoint: bigint;
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

export function select(id: bigint) {
  if (selected.has(id)) {
    let set = new Set(selected);
    set.delete(id)
    selected = set;
  } else {
    selected = new Set([...selected, id]);
  }
}

export function selectAll() {
  if (selected.size == records.length) {
    selected = new Set<bigint>();
  } else {
    let set = new Set<bigint>();
    for (let record of records) {
      set.add(record.id);
    }
    selected = set;
  }
}

export function add(record: RollcallRecord) {
  records = [record, ...records]
}
