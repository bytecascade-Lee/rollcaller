<script lang="ts">
  import {onMount} from "svelte";
  import {check, type Update} from "@tauri-apps/plugin-updater";
  import {relaunch} from "@tauri-apps/plugin-process";
  import {
    CircleNotchIcon,
    CloudArrowDownIcon,
    CloudCheckIcon,
    CloudIcon,
    CloudWarningIcon,
    CloudXIcon,
  } from "phosphor-svelte";
  import MarkdownView from "$components/common/MarkdownView.svelte";

  type UpdaterStatus =
    | "idle"
    | "checking"
    | "available"
    | "downloading"
    | "downloaded"
    | "error";

  /** 当前状态：决定图标、title 与点击行为 */
  let status = $state<UpdaterStatus>("idle");
  /** 检查到的最新版本信息（含下载/安装句柄） */
  let update = $state<Update | null>(null);
  /** 下载/安装失败信息 */
  let error = $state("");
  /** 是否已完成下载（区分下载失败后重试目标是下载还是安装） */
  let downloaded = $state(false);
  /** 安装中（禁用按钮，防重复点击） */
  let installing = $state(false);

  /** 下载进度 */
  let downloadedBytes = $state(0);
  let totalBytes = $state(0);
  let percent = $derived(
    totalBytes > 0 ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)) : 0
  );

  /** 模态弹窗开关（点击周围阴影关闭） */
  let dialogOpen = $state(false);

  onMount(() => {
    // 启动时静默检查一次
    void checkForUpdates();
  });

  /** 检查更新：失败静默回到 idle（无网络 / 尚未发布 latest.json 等） */
  async function checkForUpdates() {
    if (status === "checking") return;
    status = "checking";
    try {
      const result = await check();
      if (result) {
        update = result;
        downloaded = false;
        error = "";
        downloadedBytes = 0;
        totalBytes = 0;
        status = "available";
      } else {
        status = "idle";
      }
    } catch {
      status = "idle";
    }
  }

  /** 下载新版本。后台进行：关闭弹窗不中断下载，完成后按钮转为就绪态 */
  async function startDownload() {
    if (!update || status === "downloading") return;
    status = "downloading";
    error = "";
    downloaded = false;
    downloadedBytes = 0;
    totalBytes = 0;
    try {
      await update.download((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
        } else if (event.event === "Finished") {
          downloaded = true;
        }
      });
      status = "downloaded";
    } catch (e) {
      status = "error";
      error = e instanceof Error ? e.message : String(e);
    }
  }

  /** 安装并重启（仅下载完成后可用） */
  async function installAndRestart() {
    if (!update || !downloaded || installing) return;
    installing = true;
    try {
      await update.install();
      await relaunch();
    } catch (e) {
      installing = false;
      status = "error";
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function closeDialog() {
    dialogOpen = false;
  }

  /** 按钮点击分流：idle 触发检查；其余状态打开弹窗 */
  function onButtonClick() {
    if (status === "checking") return;
    if (status === "idle") {
      void checkForUpdates();
      return;
    }
    dialogOpen = true;
  }

  /** 弹窗底部「重试」：下载失败重下，安装失败重装 */
  function retry() {
    if (downloaded) {
      void installAndRestart();
    } else {
      void startDownload();
    }
  }

  const dialogTitle = $derived(
    status === "downloaded"
      ? "更新已就绪"
      : status === "downloading"
        ? "正在下载更新"
        : status === "error"
          ? "更新失败"
          : "发现新版本"
  );
</script>

<button
  aria-label={status === "checking"
    ? "检查更新中"
    : status === "available"
      ? "发现新版本"
      : status === "downloading"
        ? "正在下载"
        : status === "downloaded"
          ? "新版本已就绪，重启以应用更新"
          : status === "error"
            ? "更新失败，点击重试"
            : "检查更新"}
  class="icon-button {status === 'available' || status === 'downloading' ? 'primary' : status === 'downloaded' ? 'success' : status === 'error' ? 'error' : ''}"
  title={status === "checking"
    ? "检查更新中"
    : status === "available"
      ? "发现新版本"
      : status === "downloading"
        ? "正在下载"
        : status === "downloaded"
          ? "新版本已就绪，重启以应用更新"
          : status === "error"
            ? "更新失败，点击重试"
            : "检查更新"}
  onclick={onButtonClick}
>
  {#if status === "checking"}
    <CircleNotchIcon size="16" style="animation: spin 1.5s linear infinite" weight="bold"/>
  {:else if status === "available"}
    <CloudWarningIcon size="16" weight="bold"/>
  {:else if status === "downloading"}
    <CloudArrowDownIcon
      size="16"
      style="animation: pulse 1.5s var(--transition-ease-in-out) infinite, blink 1.5s var(--transition-ease-in-out) infinite"
      weight="bold"
    />
  {:else if status === "downloaded"}
    <CloudCheckIcon size="16" weight="bold"/>
  {:else if status === "error"}
    <CloudXIcon size="16" weight="bold"/>
  {:else}
    <CloudIcon size="16" weight="bold"/>
  {/if}
</button>

{#if dialogOpen && update}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- 点击周围阴影可关闭，弹窗内部阻止冒泡 -->
  <div class="overlay" onclick={closeDialog}>
    <div class="popup updater-dialog" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-title">{dialogTitle}</h3>
      <div class="version-row">
        <span class="text-content">当前版本 <b>{update.currentVersion}</b></span>
        <span class="arrow">→</span>
        <span class="text-content">最新版本 <b>{update.version}</b></span>
      </div>

      {#if status === "downloading"}
        <div class="progress">
          <div class="progress-bar">
            <div class="progress-fill" style:width="{percent}%"></div>
          </div>
          <span class="progress-text">{percent}%</span>
        </div>
      {:else if status === "error"}
        <div class="error-text">{error}</div>
      {:else if status === "downloaded"}
        <p class="text-content">
          新版本已下载完成，点击「重启并更新」应用更新（应用将自动关闭并重新打开）。
        </p>
      {:else}
        <div class="changelog">
          <MarkdownView markdown={update.body ?? ""}/>
        </div>
      {/if}

      <div class="button-group">
        <button class="button" onclick={closeDialog} disabled={installing}>取消</button>
        {#if status === "available"}
          <button class="button yes" onclick={() => void startDownload()}>下载</button>
        {:else if status === "downloading"}
          <button class="button yes" disabled>下载中…</button>
        {:else if status === "downloaded"}
          <button
            class="button yes"
            onclick={() => void installAndRestart()}
            disabled={installing}
          >
            {installing ? "重启中…" : "重启并更新"}
          </button>
        {:else if status === "error"}
          <button class="button warn" onclick={retry}>重试</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .updater-dialog {
    min-width: 26rem;
    max-width: 34rem;
  }

  .version-row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding-bottom: var(--space-xs);
  }

  .arrow {
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

  .progress {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) 0;
  }

  .progress-bar {
    flex: 1;
    height: 0.5rem;
    background: var(--color-surface);
    border-radius: var(--radius-round);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-primary);
    border-radius: var(--radius-round);
    transition: width 150ms var(--transition-ease);
  }

  .progress-text {
    min-width: 2.5rem;
    text-align: right;
    color: var(--text-color-secondary);
    font-size: var(--font-size-xs);
  }

  .error-text {
    color: var(--color-error);
    padding: var(--space-xs) 0;
    word-break: break-all;
  }
</style>
