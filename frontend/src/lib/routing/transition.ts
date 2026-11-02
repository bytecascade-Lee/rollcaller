/**
 * 转换语义：新旧两条链 → 关哪些、开哪些。
 *
 * 纯函数，不碰注册表也不碰组件状态 —— 于是"点一下到底发生了什么"可以在单测里问出来，
 * 而不用跑起整个窗口。
 */
import type {AppEntryId} from "$routing/app.entries.generated";
import type {EntryRegistry} from "$routing/registry";

/** 一次转换要做的事；`replay` 非空表示整条链没变，只补一次目标的 open */
export type TransitionPlan = {
  close: AppEntryId[];
  open: AppEntryId[];
  replay: AppEntryId | null;
};

/** 共同前缀长度 */
function commonPrefix(prev: readonly AppEntryId[], next: readonly AppEntryId[]): number {
  let shared = 0;
  while (shared < prev.length && shared < next.length && prev[shared] === next[shared]) {
    shared += 1;
  }
  return shared;
}

/**
 * 按共同前缀 diff。
 *
 * | 场景                              | 结果                                  |
 * |-----------------------------------|---------------------------------------|
 * | `edit-student` → `delete-student` | 公共前缀含卡片 ⇒ 卡片纹丝不动         |
 * | 切到无关卡片                      | 前缀为空 ⇒ 全关再开                   |
 * | 重放同一地址                      | `replay` 非空 ⇒ 补一次 open           |
 * | 退回卡片（`edit` → `card`）        | `replay` 是**平面**，见下             |
 *
 * 第三格是最反直觉的一处：因为**关闭也回退地址**，"地址停在目标、浮层早已关闭"是常见状态，
 * 没有这条补充，用户第二次点同一个入口会毫无反应。
 *
 * 第四格里 `replay` 落在卡片上：卡片没有 handler（只有叠加层注册，见 `App.svelte` 的完整性
 * 断言），于是 `registry.open` 是**预期的空操作** —— 不是漏了判断。
 */
export function planTransition(
  prev: readonly AppEntryId[],
  next: readonly AppEntryId[],
): TransitionPlan {
  const shared = commonPrefix(prev, next);
  return {
    close: prev.slice(shared).reverse(),
    open: next.slice(shared),
    replay: shared === next.length ? (next[next.length - 1] ?? null) : null,
  };
}

/** 落地只需要这两件事。收窄后不必知道注册表的载荷泛型，依赖面也从"整张表"缩到"两个方法" */
export type LandingSink = Pick<EntryRegistry<AppEntryId, never>, "open" | "close">;

/**
 * 执行转换：逆序关、顺序开（载荷只交给**最深**的那一个），返回新链。
 *
 * 祖先段一律不带载荷：它们是"铺路"用的平面，不是这次跳转的目标。
 */
export function applyTransition(
  sink: LandingSink,
  prev: readonly AppEntryId[],
  next: readonly AppEntryId[],
  payload?: unknown,
): AppEntryId[] {
  const plan = planTransition(prev, next);

  for (const id of plan.close) {
    sink.close(id);
  }
  for (const id of plan.open) {
    sink.open(id, id === plan.open[plan.open.length - 1] ? payload : undefined);
  }
  if (plan.replay !== null) {
    sink.open(plan.replay, payload);
  }
  return [...next];
}
