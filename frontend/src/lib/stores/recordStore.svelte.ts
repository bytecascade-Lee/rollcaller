import { invoke } from "@tauri-apps/api/core";
import type { RollcallRecord } from "$types/RollcallRecord";

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
            await invoke<RollcallRecord[]>("list_all_records").then((result) => {
                this.#records = result;
            });

            if (this.#boundaryPoint != 0n) return;
            let maxId = 0n;
            for (const record of this.#records) {
                if (maxId < record.id) {
                    maxId = record.id;
                }
            }
            this.#boundaryPoint = maxId;
        } finally {
            this.#isLoading = false;
        }
    }

    upsert(record: RollcallRecord) {
        const index = this.#records.findIndex((s) => s.id === record.id);
        if (index >= 0) {
            this.#records = [
                ...this.#records.slice(0, index),
                record,
                ...this.#records.slice(index + 1)
            ];
        } else {
            this.#records = [...this.#records, record];
        }
    }

    remove(ids: bigint[]) {
        this.#records = this.#records.filter((s) => !ids.includes(s.id));
    }
}

export const recordStore = new RecordStore();
