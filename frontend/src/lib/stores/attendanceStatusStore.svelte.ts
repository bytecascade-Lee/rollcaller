import {AttendanceStatusCommand} from "$commands";
import type {AttendanceStatus} from "$types";

class AttendanceStatusStore {
  #statuses = $state<Map<number, AttendanceStatus>>(new Map());
  #validStatusIds = $derived(Array.from(this.#statuses.values())
    .filter(status => status.id !== 0)
    .map(status => status.id)
    .sort((a, b) => a - b));
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

  get validStatusIds() {
    return this.#validStatusIds;
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

  nextStatus(id: number) {
    if (id === 0) {
      return this.attendanceStatus(1);
    }
    // 查找当前状态在数组中的位置
    const currentIndex = this.validStatusIds.findIndex(n => n == id);
    // 如果没找到当前状态，返回第一个有效状态
    if (currentIndex === -1) {
      return this.attendanceStatus(1);
    }
    // 获取下一个状态（如果当前是最后一个，则循环到第一个）
    const nextIndex = (currentIndex + 1) % this.validStatusIds.length;
    return this.attendanceStatus(this.#validStatusIds[nextIndex]);
  }
}

export const attendanceStatusStore = new AttendanceStatusStore();
