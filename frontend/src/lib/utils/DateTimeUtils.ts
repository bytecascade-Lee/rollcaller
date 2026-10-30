/**
 * 时间来源有两种形态：
 * - `string`：后端 IPC 与库里的规范 ISO 8601 时间字符串
 * - `number`：历史遗留的毫秒时间戳（如文件名、旧缓存）
 *
 * 展示一律按**本地时区**。
 * 排序直接比较 ISO 串的字典序即可。
 */
export function format(value: string | number): string {
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "-";

  const pad = (n: number) => String(n).padStart(2, '0');

  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}
