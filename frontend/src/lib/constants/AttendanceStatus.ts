export const STATUS_MAP = {0: "缺勤", 1: "出勤", 2: "迟到", 3: "早退", 4: "请假",};

export function statusText(code: number) {
  return STATUS_MAP[code as keyof typeof STATUS_MAP] ?? `未知(${code})`;
}

/**
 * 出勤状态样式（业务数据，取自 Open Props 色板）
 * 组件经 style: 指令注入，不写死在 CSS 中。
 */
export const STATUS_COLORS: Record<number, {bg: string; color: string}> = {
  0: {bg: "#e03131", color: "#ffffff"}, // 缺勤 红
  1: {bg: "#37b24d", color: "#ffffff"}, // 出勤 绿
  2: {bg: "#e8590c", color: "#ffffff"}, // 迟到 橙
  3: {bg: "#b8860b", color: "#ffffff"}, // 早退 黄
  4: {bg: "#1971c2", color: "#ffffff"}, // 请假 蓝
};

export const STATUS_DEFAULT_COLOR = {bg: "var(--color-page)", color: "var(--text-color-content)"};
