export function clickOutside(element: HTMLElement, options: {
  callback: () => void;
  exclude?: HTMLElement | HTMLElement[]
}) {
  const { callback, exclude } = options;

  function onClick(event: MouseEvent) {
    const target = event.target as HTMLElement;

    // 检查是否点击在弹出层内部
    if (element.contains(target)) {
      return;
    }

    // 检查是否点击在排除元素上
    if (exclude) {
      const excludes = Array.isArray(exclude) ? exclude : [exclude];
      for (const el of excludes) {
        if (el && el.contains(target)) {
          return;  // 点击了排除元素，不触发关闭
        }
      }
    }

    // 点击在外部且不在排除列表中，触发关闭
    callback();
  }

  document.body.addEventListener('click', onClick);

  return {
    destroy() {
      document.body.removeEventListener('click', onClick);
    }
  };
}
