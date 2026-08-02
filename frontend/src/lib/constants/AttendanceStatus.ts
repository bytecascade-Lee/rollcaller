export const STATUS_MAP = {0: "缺勤", 1: "出勤", 2: "迟到", 3: "早退", 4: "请假",};

export function statusText(code: number) {
  return STATUS_MAP[code as keyof typeof STATUS_MAP] ?? `未知(${code})`;
}

/**
 * 出勤状态样式（业务数据，取自 Open Props 色板）
 * 组件经 style: 指令注入，不写死在 CSS 中。
 */
export const STATUS_COLORS: Record<number, {bg: string; color: string}> = {
  0: {bg: "#e03131", color: "#fff5f5"}, // 缺勤 红
  1: {bg: "#37b24d", color: "#ebfbee"}, // 出勤 绿
  2: {bg: "#f76707", color: "#fff4e6"}, // 迟到 橙
  3: {bg: "#f59f00", color: "#fff9db"}, // 早退 黄
  4: {bg: "#1971c2", color: "#e7f5ff"}, // 请假 蓝
};

export const STATUS_DEFAULT_COLOR = {bg: "var(--app-color-surface-muted)", color: "var(--app-color-text-muted)"};
