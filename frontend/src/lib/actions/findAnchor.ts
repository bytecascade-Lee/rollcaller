/**
 * 按入口名找触发元素。
 *
 * 位置是渲染上下文，不是数据：既不进载荷（DOM 引用不可序列化），也不走 props
 * （一个页面里两个浮层会共用同一个变量，先点 A 再点 B 就错位）。触发元素自己用
 * `data-anchor={AppEntry.x}` 打标，浮层按**自己的身份**来查。
 *
 * 查不到返回 null —— 深链、跨窗口投递、卡片还没激活（`.content > *` 是 `display: none`，
 * 此时 `getClientRects()` 为空）都算查不到，调用方退回居中。**不能只看元素存在**：
 * 未布局的元素 rect 全为 0，会把浮层摆到视口左上角。
 */
export function findAnchor(entry: string): HTMLElement | null {
  const element = document.querySelector<HTMLElement>(`[data-anchor="${entry}"]`);
  return element && element.getClientRects().length > 0 ? element : null;
}
