<script lang="ts">
  import {STATE_CONFIG, TriSwitchState} from "$types/TriSwitchState";

  let {value}: { value: TriSwitchState } = $props();

  // 获取当前状态的配置
  let config = $derived(STATE_CONFIG[value])
</script>

<div
  class="track"
  style:background={config.trackBg}
>
  <!-- svelte-ignore element_invalid_self_closing_tag -->
  <div
    class="thumb"
    style:left="{config.thumbLeft}px"
  />
</div>

<style>
  .thumb {
    position: absolute;
    top: 2px;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-xl);
    background: var(--color-card);
    transition: left var(--transition-duration-md) cubic-bezier(0.34, 1.56, 0.64, 1);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

</style>
