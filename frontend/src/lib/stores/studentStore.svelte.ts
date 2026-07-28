import {invoke} from "@tauri-apps/api/core";
import type {StudentTable} from "$types/StudentTable";

export let students = $state<StudentTable[]>([]);
export let selected = $state<Set<bigint>>(new Set());
export let isLoading = $state(false);

export async function load() {
  isLoading = true;
  try {
    students = await invoke<StudentTable[]>("list_all_students");
  } finally {
    isLoading = false;
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
  if (selected.size == students.length) {
    selected = new Set<bigint>();
  } else {
    let set = new Set<bigint>();
    for (let student of students) {
      set.add(student.id);
    }
    selected = set;
  }
}

export function upsert(student: StudentTable) {
  const index = students.findIndex((s) => s.id === student.id);
  if (index >= 0) {
    students = [...students.slice(0, index), student, ...students.slice(index + 1)];
  } else {
    students = [...students, student];
  }
}

export function remove(ids: (number | bigint)[]) {
  students = students.filter((s) => !ids.includes(s.id));
}
