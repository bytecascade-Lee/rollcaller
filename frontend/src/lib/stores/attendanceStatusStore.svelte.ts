import {AttendanceStatusCommand} from "$commands";
import type {AttendanceStatus} from "$types";

class AttendanceStatusStore {
  #statuses = $state<Map<number, AttendanceStatus>>(new Map());
  #isLoading = $state<boolean>(false);
  fallback: AttendanceStatus = {
    id: 0,
    name: "FALLBACK",
    background: "#333333",
    color: "#f0f0f0",
    remark: null,
    is_deleted: 0,
    deleted_at: null
  };

  attendanceStatusMap() {
    return this.#statuses;
  }

  attendanceStatus(id: number) {
    let status = this.#statuses.get(id);
    return status ? status : this.fallback;
  }

  isLoading() {
    return this.#isLoading;
  }

  async load() {
    this.#isLoading = true;
    try {
      this.#statuses = await AttendanceStatusCommand.list()
    } catch (e) {
      alert(e);
    } finally {
      this.#isLoading = false;
    }
  }
}

export const attendanceStatusStore = new AttendanceStatusStore();
