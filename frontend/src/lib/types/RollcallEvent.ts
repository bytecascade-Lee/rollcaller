/**
 * 点名面板状态机事件
 *
 * - Start: 开始点名（仅 Idle 下有效）
 * - AnimateDone: 动画自然结束（连续点名自动触发）
 * - UserStop: 用户点击停止（标记 pendingStop，不打断当前事务）
 * - PickDone: 随机选人完成
 * - SaveSuccess: 记录写入数据库成功
 * - SaveFailed: 记录写入数据库失败
 * - ShowDone: 结果展示结束
 */
export enum RollcallEvent {
  Start = "Start",
  AnimateDone = "AnimateDone",
  UserStop = "UserStop",
  PickDone = "PickDone",
  SaveSuccess = "SaveSuccess",
  SaveFailed = "SaveFailed",
  ShowDone = "ShowDone",
}
