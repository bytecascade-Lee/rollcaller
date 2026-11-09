import type {UpdateView} from "$types/UpdateView";

/**
 * 视图迁移是否应该「自己弹出来」
 *
 * 值得打断用户的事只有一件：**critical** 的候选更新（用户必须看到）。其余一律只改标题栏图标——
 * 自动检查是开机就跑的动作，用户没问过；下载是用户自己点出来的，后台下载完成也不该抢焦点。
 *
 * 唯一的例外是「用户已经在流程里」的失败：下载 / 安装出错必须可见，而检查失败（多半是自动检查）
 * 只留标题栏的叉子图标，不打扰。
 *
 * 纯函数的原因：`environment: node` 下 `.svelte.ts` 是 server 编译、`$effect` 永不运行，
 * 规则抽出来才能被单测覆盖。
 *
 * @param prev 上一次视图的 `status`（`null` = 首次观察，如启动恢复现场）
 * @param next 本次视图
 */
export function shouldAutoOpen(prev: UpdateView["status"] | null, next: UpdateView): boolean {
  if (prev === next.status) return false;
  if (hasCandidate(next)) return next.data.severity === "critical";
  if (next.status === "error") return next.data.type !== "check";
  return false;
}

/** 带候选更新的三种状态：它们描述的是同一个候选，字段一起走 */
function hasCandidate(
  view: UpdateView,
): view is Extract<UpdateView, {status: "available" | "downloading" | "downloaded"}> {
  return view.status === "available" || view.status === "downloading" || view.status === "downloaded";
}
