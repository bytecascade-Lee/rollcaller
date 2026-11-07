/** 没有可见触发元素时的兜底位置：视口居中（`updatePosition` 只负责"锚在某个元素下方"） */
export const POPUP_CENTERED = "position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%)";

export function updatePosition(anchor: HTMLElement) {
  const rect = anchor.getBoundingClientRect();
  const popupWidth = Math.max(rect.width, 280);
  const viewportWidth = window.innerWidth;
  const margin = 8;

  // 默认左对齐
  let left = rect.left;

  // 只有右侧空间不够时才调整
  if (left + popupWidth + margin > viewportWidth) {
    left = viewportWidth - popupWidth - margin;
  }

  // 左侧边界保护
  left = Math.max(margin, left);

  return `position: fixed; top: ${rect.bottom + 6}px; left: ${left}px`;
}
