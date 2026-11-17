<script lang="ts">
  import {onMount, tick} from "svelte";
  import MarkdownView from "$components/common/MarkdownView.svelte";
  import {AppInfoCommand} from "$commands";
  import {updateStore, updateUiStore} from "$stores";
  import {UpdateOpen} from "$utils";
  import type {UpdateView} from "$types";

  // 单一数据源
  let view = $derived(updateStore.view);
  /** 三态（available / downloading / downloaded）共用一个候选，字段一起取 */
  let candidate = $derived(
    view.status === "available" || view.status === "downloading" || view.status === "downloaded" ? view.data : null,
  );
  let info = $derived(candidate?.info ?? null);
  let severity = $derived(candidate?.severity ?? null);
  let errorMessage = $derived(view.status === "error" ? view.data.message : null);
  // 后端错误已合并为 { type, message }：type = 失败阶段 = 重试应调的命令
  let retry = $derived(view.status === "error" ? view.data.type : null);
  // 下载进度只存在于 store 的进度槽（Downloading 期有效；后端 Downloading 视图不再带数字）
  let progress = $derived(updateStore.progress);
  let downloaded = $derived(progress.downloaded);
  let total = $derived(progress.total);
  let hasTotal = $derived(total !== null && total > 0);
  let percent = $derived(total !== null && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0);
  /** 强制更新：critical 的候选，弹窗不给任何关闭手段 */
  let forced = $derived(updateUiStore.forced);

  /**
   * 解释性文案：`reason` 的用武之地——它说明「这条更新为什么被放行」
   *
   * 只解释用户可能想不通的那两种：critical 为什么不能跳过、「重要」为什么绕过了门槛。
   * `reason = "newer"` 的 important 不需要解释（本来就没忽略什么），只显示徽标。
   */
  let hint = $derived.by(() => {
    if (!candidate) return null;
    if (candidate.severity === "critical") {
      return candidate.reason === "force"
        ? "此更新为紧急更新，已忽略你设置的版本更新门槛，必须完成安装后才能继续使用。"
        : "此更新为紧急更新，必须完成安装后才能继续使用。";
    }
    if (candidate.severity === "important" && candidate.reason === "exempt") {
      return "此更新被标记为「重要」，已忽略你设置的版本更新门槛。";
    }
    return null;
  });

  let currentVersion = $state("");
  let installing = $state(false);
  let dialogEl = $state<HTMLDivElement | null>(null);
  let primaryEl = $state<HTMLButtonElement | null>(null);

  // 上一次视图的阶段，`null` = 启动恢复现场
  let prevStatus: UpdateView["status"] | null = null;

  $effect(() => {
    const next = view;
    if (UpdateOpen.shouldAutoOpen(prevStatus, next)) updateUiStore.show();
    prevStatus = next.status;
  });

  // 打开时把焦点放进弹窗：优先主按钮，没有主按钮（如下载中）就落在弹窗本身。
  // 背景的不可聚焦由 `App.svelte` 的 `inert` 保证，这里只负责"进来"。
  $effect(() => {
    if (!updateUiStore.open) return;
    void tick().then(() => (primaryEl ?? dialogEl)?.focus());
  });

  /** Esc 关闭；强制更新期间 `hide()` 是 no-op，所以这里不用再判一次 */
  function onKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape" || !updateUiStore.open) return;
    updateUiStore.hide();
  }

  function checkNow() {
    if (view.status === "checking" || view.status === "downloading") return;
    void updateStore.check();
  }

  async function download() {
    if (view.status !== "available") return;
    await updateStore.download();
  }

  async function cancelDownload() {
    await updateStore.cancel();
    updateUiStore.hide();
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
    const info = await AppInfoCommand.app_info();
    currentVersion = info.version;
  });
</script>

<svelte:window onkeydown={onKeydown} />

{#if updateUiStore.open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => updateUiStore.hide()}>
    <div
      aria-labelledby="updater-dialog-title"
      aria-modal="true"
      bind:this={dialogEl}
      class="popup"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
    >
      <div class="popup-header">
        <h3 class="text-title" id="updater-dialog-title">
          {#if view.status == "checking"}
            正在检查更新...
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

      {#if hint && (view.status == "available" || view.status == "downloaded")}
        <p class="text-content hint">{hint}</p>
      {/if}

      {#if view.status == "available"}
        <span class="text-content">发布于：{info?.date}</span>
        <div class="changelog">
          <MarkdownView markdown={info?.notes ?? ""} />
        </div>
      {:else if view.status == "downloading"}
        {#if hasTotal}
          <div class="progress">
            <div
              aria-label="下载进度"
              aria-valuemax="100"
              aria-valuemin="0"
              aria-valuenow={percent}
              class="progress-bar"
              role="progressbar"
            >
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
        {#if forced}
          <p class="text-content hint">更新中，无法取消。</p>
        {/if}
      {:else if view.status == "downloaded"}
        <p class="text-content">
          新版本 v{info?.version} 已下载完成，点击「重启并更新」应用更新（应用将自动关闭并重新打开）。
        </p>
      {:else if view.status == "error"}
        <div aria-live="polite" class="text-content" style="color: var(--color-error)">{errorMessage}</div>
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

      <!-- 入口拒绝类错误提示 -->
      {#if updateStore.lastError}
        <p class="text-content" style="margin-top: var(--space-xs); color: var(--color-error);">
          {updateStore.lastError}
        </p>
      {/if}

      <div class="button-group">
        {#if forced}
          <!-- 强制更新：没有关闭 / 重新检查 / 取消，只有把更新做完这一条路 -->
          {#if view.status == "available"}
            <button bind:this={primaryEl} class="button yes" onclick={() => void download()}>立即更新</button>
          {:else if view.status == "downloaded"}
            <button
              bind:this={primaryEl}
              class="button yes"
              disabled={installing}
              onclick={() => void installAndRestart()}
            >
              {installing ? "重启中..." : "重启并更新"}
            </button>
          {/if}
        {:else}
          <button class="button" disabled={installing} onclick={() => updateUiStore.hide()}>
            {#if view.status == "downloading"}
              后台下载
            {:else if view.status == "downloaded"}
              稍后
            {:else}
              关闭
            {/if}
          </button>

          {#if view.status == "available"}
            <button class="button warn" onclick={checkNow}>重新检查</button>
            <button bind:this={primaryEl} class="button yes" onclick={() => void download()}>下载</button>
          {:else if view.status == "downloading"}
            <button class="button warn" onclick={() => void cancelDownload()}>取消下载</button>
          {:else if view.status == "downloaded"}
            <button
              bind:this={primaryEl}
              class="button yes"
              disabled={installing}
              onclick={() => void installAndRestart()}
            >
              {installing ? "重启中..." : "重启并更新"}
            </button>
          {:else if view.status == "error"}
            <button bind:this={primaryEl} class="button warn" onclick={retryAction}>重试</button>
          {:else if view.status == "upToDate" || view.status == "idle"}
            <button bind:this={primaryEl} class="button yes" onclick={checkNow}>检查更新</button>
          {/if}
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

  .hint {
    color: var(--text-color-secondary);
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
