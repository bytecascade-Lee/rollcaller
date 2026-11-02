/** 地址栈：`entries` 为历史，`index` 为栈顶下标 */

export type History<S> = {
  entries: readonly S[];
  index: number;
};

/** 追加一条并裁掉前进段 */
export function pushAt<S>(state: History<S>, next: S): History<S> {
  const entries = [...state.entries.slice(0, state.index + 1), next];
  return {entries, index: entries.length - 1};
}

/** 重置为单条：跨窗口投递与冷启动载荷是「送达」，不进本窗口的点击历史 */
export function resetTo<S>(next: S): History<S> {
  return {entries: [next], index: 0};
}

/** 移动 `delta` 格后的下标；越界返回 `null`（`canGoBack` / `canGoForward` 也用它判定） */
export function stepIndex<S>(state: History<S>, delta: number): number | null {
  const index = state.index + delta;
  return index >= 0 && index < state.entries.length ? index : null;
}
