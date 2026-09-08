<script lang="ts">
  import {onMount} from "svelte";
  import {
    CircleNotchIcon,
    CloudArrowDownIcon,
    CloudCheckIcon,
    CloudIcon,
    CloudWarningIcon,
    CloudXIcon,
  } from "phosphor-svelte";
  import MarkdownView from "$components/common/MarkdownView.svelte";
  import {AppInfoCommand} from "$commands";
  import {updateStore} from "$stores/UpdateStore.svelte";

  let {autoCheck = true}: { autoCheck?: boolean } = $props();

  // 单一数据源：后端裁剪视图（命令返回 / 广播同类型），组件只渲染，不推断阶段
  let view = $derived(updateStore.view);
  let currentVersion = $state("");
  let isVisible = $state(false);
  let installing = $state(false);
  let previousStatus = $state<string | null>(null);

  // [TODO]：当前仅主窗口（标题栏）渲染本组件，autoCheck 亦只在此发生；
  // 后期设置窗口加入"检查更新"入口时，重审多窗口各自的 autoCheck 触发与
  // UpdateStore 多实例幂等订阅（后端广播本就喂全窗口）。

  // 从 tagged union 里安全取展示字段
  let info = $derived(
    (view.status === "available" || view.status === "downloading" || view.status === "downloaded")
      ? view.data.info
      : null,
  );
  let severity = $derived(
    (view.status === "available" || view.status === "downloaded") ? view.data.severity : null,
  );
  let errorMessage = $derived(view.status === "error" ? view.data.message : null);
  // 后端错误已合并为 { type, message }：type = 失败阶段 = 重试应调的命令
  let retry = $derived(view.status === "error" ? view.data.type : null);
  // 下载进度只存在于 store 的进度槽（Downloading 期有效；后端 Downloading 视图不再带数字）
  let progress = $derived(updateStore.progress);
  let downloaded = $derived(progress.downloaded);
  let total = $derived(progress.total);
  let hasTotal = $derived(total !== null && total > 0);
  let percent = $derived(
    total !== null && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0,
  );

  // 状态变化时的自动弹窗：检查发现新版本 / 下载完成 / 出错时主动提示
  $effect(() => {
    const s = view.status;
    if (s === previousStatus) return;
    const prev = previousStatus;
    previousStatus = s;
    if (
      (s === "available" && prev === "checking") ||
      (s === "downloaded" && prev === "downloading") ||
      (s === "error" && prev !== "error")
    ) {
      isVisible = true;
    }
  });

  async function checkForUpdates() {
    if (view.status === "checking" || view.status === "downloading") return;
    isVisible = true;
    await updateStore.check();
  }

  async function download() {
    if (view.status !== "available") return;
    await updateStore.download();
  }

  async function cancelDownload() {
    await updateStore.cancel();
    isVisible = false;
  }

  async function installAndRestart() {
    if (view.status !== "downloaded" || installing) return;
    installing = true;
    try {
      await updateStore.install();
    } finally {
      installing = false;
    }
  }

  function retryAction() {
    if (retry === "check") void updateStore.check();
    else if (retry === "download") void updateStore.download();
    else if (retry === "install") void updateStore.install();
  }

  onMount(async () => {
    // 订阅广播 + 恢复现场（Idle 时自动检查一次）
    await updateStore.init();
    if (autoCheck && updateStore.view.status === "idle") {
      await updateStore.check();
    }
    const info = await AppInfoCommand.app_info();
    currentVersion = info.version;
  });
</script>

<!-- 标题栏图标：按视图状态切换 -->
{#if view.status == "idle"}
  <button
    aria-label="检查更新"
    class="icon-button"
    title="检查更新"
    onclick={checkForUpdates}
  >
    <CloudIcon size="16" weight="bold"/>
  </button>
{:else if view.status == "checking"}
  <button aria-label="正在检查更新" class="icon-button" title="正在检查更新…">
    <CircleNotchIcon size="16" style="animation: spin 1.5s linear infinite" weight="bold"/>
  </button>
{:else if view.status == "available"}
  <button
    aria-label="发现新版本"
    class="icon-button primary"
    title="发现新版本"
    onclick={() => (isVisible = true)}
  >
    <CloudWarningIcon size="16" weight="bold"/>
  </button>
{:else if view.status == "upToDate"}
  <button
    aria-label="已是最新版本"
    class="icon-button"
    title="已是最新版本"
    onclick={() => (isVisible = true)}
  >
    <CloudCheckIcon size="16" weight="bold"/>
  </button>
{:else if view.status == "downloading"}
  <button
    aria-label="正在下载更新"
    class="icon-button primary"
    title="正在下载更新"
    onclick={() => (isVisible = true)}
  >
    <CloudArrowDownIcon
      size="16"
      style="animation: pulse 1.5s var(--transition-ease-in-out) infinite, blink 1.5s var(--transition-ease-in-out) infinite"
      weight="bold"
    />
  </button>
{:else if view.status == "downloaded"}
  <button
    aria-label="更新已就绪"
    class="icon-button success"
    title="更新已就绪，点击重启应用"
    onclick={() => (isVisible = true)}
  >
    <CloudCheckIcon size="16" weight="bold"/>
  </button>
{:else if view.status == "error"}
  <button
    aria-label="更新出错"
    class="icon-button error"
    title="更新出错"
    onclick={() => (isVisible = true)}
  >
    <CloudXIcon size="16" weight="bold"/>
  </button>
{/if}

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => (isVisible = false)}>
    <div class="popup" onclick={(e) => e.stopPropagation()}>
      <div class="popup-header">
        <h3 class="text-title">
          {#if view.status == "checking"}
            正在检查更新…
          {:else if view.status == "available"}
            发现新版本 v{info?.version}
          {:else if view.status == "upToDate"}
            已是最新版本
          {:else if view.status == "downloading"}
            正在下载更新
          {:else if view.status == "downloaded"}
            更新已就绪
          {:else if view.status == "error"}
            更新出错
          {:else}
            检查更新
          {/if}
        </h3>
        {#if severity == "critical"}
          <span class="severity-badge critical">紧急更新</span>
        {:else if severity == "important"}
          <span class="severity-badge important">重要更新</span>
        {/if}
      </div>

      {#if view.status == "available"}
        <span class="text-content">发布于：{info?.date}</span>
        <div class="changelog">
          <MarkdownView markdown={info?.notes ?? ""}/>
        </div>
      {:else if view.status == "downloading"}
        {#if hasTotal}
          <div class="progress">
            <div class="progress-bar">
              <div class="progress-fill" style:width="{percent}%"></div>
            </div>
            <span class="progress-text">{percent}%</span>
          </div>
          <p class="text-content" style="margin-top: var(--space-xs);">
            正在下载 v{info?.version}（{Math.round(downloaded / 1048576)} MB / {Math.round((total ?? 0) / 1048576)} MB），请稍候…
          </p>
        {:else}
          {#if downloaded > 0}
            <p class="text-content" style="margin-top: var(--space-xs);">
              正在下载 v{info?.version}（已下载 {Math.round(downloaded / 1048576)} MB / 总大小未知），请稍候…
            </p>
          {:else}
            <p class="text-content" style="margin-top: var(--space-xs);">
              正在下载 v{info?.version}，请稍候…
            </p>
          {/if}
        {/if}
      {:else if view.status == "downloaded"}
        <p class="text-content">
          新版本 v{info?.version} 已下载完成，点击「重启并更新」应用更新（应用将自动关闭并重新打开）。
        </p>
      {:else if view.status == "error"}
        <div class="text-content" style="color: var(--color-error)">{errorMessage}</div>
        <p class="text-content" style="margin-top: var(--space-xs);">
          {#if retry == "check"}
            检查更新失败，可重试。
          {:else if retry == "download"}
            下载失败，可重试下载。
          {:else if retry == "install"}
            应用更新失败，可重试安装。
          {:else}
            请稍后再试。
          {/if}
        </p>
      {:else if view.status == "upToDate"}
        <p class="text-content">当前版本 <b>{currentVersion}</b> 已是最新版本。</p>
      {/if}

      <!-- 入口拒绝类错误（如开发模式）提示 -->
      {#if updateStore.lastError}
        <p class="text-content" style="margin-top: var(--space-xs); color: var(--color-error);">
          {updateStore.lastError}
        </p>
      {/if}

      <div class="button-group">
        <button class="button" onclick={() => (isVisible = false)} disabled={installing}>
          {#if view.status == "downloading"}
            后台下载
          {:else}
            关闭
          {/if}
        </button>

        {#if view.status == "available"}
          <button class="button warn" onclick={checkForUpdates}>重新检查</button>
          <button class="button yes" onclick={() => void download()}>下载</button>
        {:else if view.status == "downloading"}
          <button class="button warn" onclick={() => void cancelDownload()}>取消下载</button>
        {:else if view.status == "downloaded"}
          <button class="button yes" onclick={() => void installAndRestart()} disabled={installing}>
            {installing ? "重启中…" : "重启并更新"}
          </button>
        {:else if view.status == "error"}
          <button class="button warn" onclick={retryAction}>重试</button>
        {:else if view.status == "upToDate" || view.status == "idle"}
          <button class="button yes" onclick={checkForUpdates}>检查更新</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .popup-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-xs);
  }

  .severity-badge {
    padding: 0 var(--space-xxs);
    border-radius: var(--radius-xxs);
    font-size: var(--font-size-xxs);
    line-height: 1.6;
    color: #fff;
  }

  .severity-badge.critical {
    background: var(--color-error);
  }

  .severity-badge.important {
    background: var(--color-warn);
  }

  .changelog {
    max-height: 16rem;
    overflow-y: auto;
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-card);
    border: var(--border-size-xxs) solid var(--border-color-3);
    border-radius: var(--radius-sm);
  }
</style>
