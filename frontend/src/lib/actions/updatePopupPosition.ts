export function updatePosition(anchor: HTMLElement)  {
  console.log('anchor 元素:', anchor);
  console.log('anchor 标签名:', anchor.tagName);
  console.log('anchor 类名:', anchor.className);
  console.log('anchor 的宽高:', anchor.offsetWidth, 'x', anchor.offsetHeight);
  console.log('anchor 的位置:', anchor.getBoundingClientRect());
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

  return `position: fixed; top: ${rect.bottom + 6}px; left: ${left}px; min-width: ${popupWidth}px;`;
}
