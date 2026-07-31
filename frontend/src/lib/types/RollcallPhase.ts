/**
 * 点名面板状态机状态
 *
 * - Idle: 空闲，等待开始
 * - Animating: 名字滚动动画中
 * - Picking: 随机选人 + 写入数据库（事务必须完整走完）
 * - Showing: 展示选中的学生
 */
export enum RollcallPhase {
  Idle = "Idle",
  Animating = "Animating",
  Picking = "Picking",
  Showing = "Showing",
}
