import {invoke} from "@tauri-apps/api/core";
import type {StudentSingleCreateResult} from "$types/StudentSingleCreateResult";
import {Student} from "$types/Student";
import type {StudentTable} from "$types/StudentTable";
import {StudentSingleUpdate} from "$types/StudentSingleUpdate";

export async function list() {
  return await invoke<StudentTable[]>("student_list");
}

export async function create(student: Student, override: boolean | null) {
  return await invoke<StudentSingleCreateResult>("student_single_create", {
    student: student,
    overwrite: override,
  })
}

export async function update(student: Student) {
  return await invoke<StudentSingleUpdate>("student_single_update", {
    student: student
  })
}

export async function remove(ids: bigint[]) {
  void await invoke("student_batch_delete", {
    ids: ids,
  })
}

export async function restore(ids: bigint[]) {
  return await invoke<StudentTable>("student_batch_restore", {
    ids: ids
  })
}

export async function expose(path: string, ids: bigint[]) {
  void await invoke<null>("student_export", {
    path: path,
    ids: ids
  })
}
