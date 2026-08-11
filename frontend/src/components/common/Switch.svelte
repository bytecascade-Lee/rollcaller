<script lang="ts">
  // 纯展示开关：状态由父组件传入（yes），点击交互由父组件通过包裹的按钮处理
  let {yes}: { yes: boolean } = $props();
</script>

<!-- 装饰性视觉元素，交互语义在父组件按钮上 -->
<div
  class="track"
  class:on={yes}
  aria-hidden="true"
>
  <!-- svelte-ignore element_invalid_self_closing_tag -->
  <div
    class="thumb"
    class:on={yes}
  />
</div>

<style>
  .track {
    position: relative;
    width: 52px;
    height: 28px;
    /* 两端半圆 + 中间矩形 */
    border-radius: 50%;
    padding: 2px;
    background: var(--color-surface);
    transition: background var(--transition-duration-md) var(--transition-ease);
    /* 点击事件由父组件按钮接收 */
    pointer-events: none;
    user-select: none;
    box-sizing: border-box;
  }

  .track.on {
    background: var(--color-primary);
  }

  .thumb {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--color-card);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
    transition: transform var(--transition-duration-md) cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  /* 52 - 2*2 padding - 24 = 24px，圆滑到右侧 */
  .thumb.on {
    transform: translateX(24px);
  }
</style>
