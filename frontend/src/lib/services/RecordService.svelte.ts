import {RollcallRecord} from "$types/RollcallRecord";
import type {RecordGroupMetaData} from "$types/RecordGroupMetaData";
import {attendanceStatusStore, recordStore} from "$stores";
import {RecordCommand} from "$commands";

class RecordService {
  #groupColors= [
    "#e94f4f",
    '#eea700',
    '#45c06e',
    '#00adcf',
    '#c442de'
  ]

  get groupColors(): string[] {
    return this.#groupColors;
  }

  group(records: RollcallRecord[]) {
    let result: RecordGroupMetaData[] = [];
    if (records.length == 0) return result;

    let lastSessionId = "";
    let groupIndex = -1
    for (let i = 0; i < records.length; i++) {
      let currentSessionId = records[i].session_id;
      if (currentSessionId != lastSessionId) {
        lastSessionId = currentSessionId;
        let rowspan = 0;
        groupIndex++;
        while (i + rowspan < records.length && currentSessionId == records[i + rowspan].session_id) {
          rowspan++
        }
        result.push({
          groupIndex: groupIndex,
          isStart: true,
          rowspan: rowspan
        })
      } else {
        result.push({
          groupIndex: groupIndex,
          isStart: false,
          rowspan: 0
        })
      }
    }
    return result;
  }

  async updateToNextStatus(id: bigint, attendanceStatusId: number) {
    let status = attendanceStatusStore.nextStatus(attendanceStatusId);
    try {
      let result = await RecordCommand.update_attendance_status([id], status ? status.id : 1);
      recordStore.upsert(result[0]);
    } catch (e) {
      alert(e)
    }
  }
}

export const recordService = new RecordService();
