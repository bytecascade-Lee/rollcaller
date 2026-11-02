/** 转换语义：新旧两条链 → 关哪些、开哪些。纯函数，可单测 */
import type {AppEntryId} from "$routing/app/entries.generated";
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
 * 按共同前缀 diff：前缀之后逆序关、顺序开；整条链相同时 `replay` 非空。
 *
 * `replay` 不能删：关闭也回退地址，「地址在目标、浮层已关」是常态，不补 `open` 则再点无反应。
 * 它落在卡片上时 `registry.open` 是预期的空操作 —— 卡片没有 handler，不是漏判
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

/** 落地只需要这两个方法，故不必携带注册表的载荷泛型 */
export type LandingSink = Pick<EntryRegistry<AppEntryId, never>, "open" | "close">;

/**
 * 执行转换：逆序关、顺序开（载荷只交给最深的一个），返回新链。
 *
 * 祖先段不带载荷：它们是铺路用的平面，不是本次跳转的目标
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
