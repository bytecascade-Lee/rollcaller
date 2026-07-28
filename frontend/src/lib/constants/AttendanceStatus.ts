export const STATUS_MAP = {0: "缺勤", 1: "出勤", 2: "迟到", 3: "早退", 4: "请假",};

export function statusText(code: number) {
  return STATUS_MAP[code as keyof typeof STATUS_MAP] ?? `未知(${code})`;
}
