import type {RollcallRecord} from "$types";
import {RecordCommand} from "$commands";

class RecordStore {
  #records = $state<RollcallRecord[]>([]);
  #boundaryPoint = $state<bigint>(0n);
  #isLoading = $state<boolean>(false);

  get records() {
    return this.#records;
  }

  get boundaryPoint() {
    return this.#boundaryPoint;
  }

  get isLoading() {
    return this.#isLoading;
  }

  async load() {
    this.#isLoading = true;
    try {
      this.#records = await RecordCommand.list();

      if (this.#boundaryPoint != 0n) return;
      let maxId = 0n;
      for (const record of this.#records) {
        if (maxId < record.id) {
          maxId = record.id;
        }
      }
      this.#boundaryPoint = maxId;
    } catch (e) {
      alert(e);
    } finally {
      this.#isLoading = false;
    }
  }

  get(id: bigint) {
    let find = this.#records.find(value => value.id == id);
    return find ? find : null
  }

  upsert(record: RollcallRecord) {
    const index = this.#records.findIndex((s) => s.id == record.id);
    if (index >= 0) {
      this.#records = [
        ...this.#records.slice(0, index),
        record,
        ...this.#records.slice(index + 1)
      ];
    } else {
      this.#records = [record, ...this.#records];
    }
  }

  remove(ids: bigint[]) {
    this.#records = this.#records.filter((s) => !ids.includes(s.id));
  }
}

export const recordStore = new RecordStore();
