import {RollcallRecord} from "$types/RollcallRecord";
import type {RecordGroupMetaData} from "$types/RecordGroupMetaData";

export const COLORS = ['#7be5d5', '#b86dff', '#d8d1d1', '#d5b136'];

export function group(records: RollcallRecord[]){
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
