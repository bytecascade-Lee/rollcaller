<script lang="ts">
  import {
    CircleNotchIcon,
    CloudArrowDownIcon,
    CloudCheckIcon,
    CloudIcon,
    CloudWarningIcon,
    CloudXIcon,
  } from "phosphor-svelte";
  import {updateStore, updateUiStore} from "$stores";

  // 渲染后端裁剪视图
  let view = $derived(updateStore.view);

  /**
   * 空闲：先开弹窗再检查，让"正在检查"可见；
   * 正在检查中：返回
   * 其余状态：只显示弹窗
   */
  function onClick() {
    if (view.status === "idle") {
      updateUiStore.show();
      void updateStore.check();
      return;
    }
    if (view.status === "checking") return;
    updateUiStore.toggle();
  }
</script>

{#if view.status == "idle"}
  <button aria-label="检查更新" class="icon-button" title="检查更新" onclick={onClick}>
    <CloudIcon size="16" weight="bold" />
  </button>
{:else if view.status == "checking"}
  <button aria-label="正在检查更新" class="icon-button" title="正在检查更新...">
    <CircleNotchIcon size="16" style="animation: spin 1.5s linear infinite" weight="bold" />
  </button>
{:else if view.status == "available"}
  <button aria-label="发现新版本" class="icon-button primary" title="发现新版本" onclick={onClick}>
    <CloudWarningIcon size="16" weight="bold" />
  </button>
{:else if view.status == "upToDate"}
  <button aria-label="已是最新版本" class="icon-button" title="已是最新版本" onclick={onClick}>
    <CloudCheckIcon size="16" weight="bold" />
  </button>
{:else if view.status == "downloading"}
  <button aria-label="正在下载更新" class="icon-button primary" title="正在下载更新" onclick={onClick}>
    <CloudArrowDownIcon
      size="16"
      style="animation: pulse 1.5s var(--transition-ease-in-out) infinite, blink 1.5s var(--transition-ease-in-out) infinite"
      weight="bold"
    />
  </button>
{:else if view.status == "downloaded"}
  <button aria-label="更新已就绪" class="icon-button success" title="更新已就绪，点击重启应用" onclick={onClick}>
    <CloudCheckIcon size="16" weight="bold" />
  </button>
{:else if view.status == "error"}
  <button aria-label="更新出错" class="icon-button error" title="更新出错" onclick={onClick}>
    <CloudXIcon size="16" weight="bold" />
  </button>
{/if}
