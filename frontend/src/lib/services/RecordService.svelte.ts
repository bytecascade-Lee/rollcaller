import {RollcallRecord} from "$types/RollcallRecord";
import type {RecordGroupMetaData} from "$types/RecordGroupMetaData";

export const COLORS = [
  "#e94f4f",
  '#eea700',
  '#45c06e',
  '#00adcf',
  '#c442de'
];

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
