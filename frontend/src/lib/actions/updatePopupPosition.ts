export function updatePosition(anchor: HTMLElement)  {
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
