import type {StudentTable} from "$types/StudentTable";
import {StudentCommand} from "$commands";

class StudentStore {
  #students = $state<StudentTable[]>([]);
  #isLoading = $state<boolean>(false);

  get students() {
    return this.#students;
  }

  get isLoading() {
    return this.#isLoading;
  }

  async load() {
    this.#isLoading = true;
    try {
      this.#students = await StudentCommand.list();
    } catch (e) {
      alert(e)
    } finally {
      this.#isLoading = false;
    }
  }

  get(id: bigint) {
    let find = this.#students.find(value => value.id == id);
    return find ? find : null
  }

  upsert(student: StudentTable) {
    const index = this.#students.findIndex((s) => s.id === student.id);
    if (index >= 0) {
      this.#students = [
        ...this.#students.slice(0, index),
        student,
        ...this.#students.slice(index + 1)
      ];
    } else {
      this.#students = [...this.#students, student];
    }
  }

  remove(ids: bigint[]) {
    this.#students = this.#students.filter((s) => !ids.includes(s.id));
  }
}

export const studentStore = new StudentStore();
