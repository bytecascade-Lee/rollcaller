/**
 * 分组元数据 - 描述原始数据中按 sessionId 分组的区间信息
 */
export type RecordGroupMetaData = {
  groupIndex: number;
  isStart: boolean;
  rowspan: number;
};
