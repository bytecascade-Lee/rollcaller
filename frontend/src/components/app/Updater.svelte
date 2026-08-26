<script lang="ts">
  import {onDestroy, onMount} from 'svelte';
  import {check, type Update} from '@tauri-apps/plugin-updater';
  import {relaunch} from '@tauri-apps/plugin-process';
  import {
    CircleNotchIcon,
    CloudArrowDownIcon,
    CloudCheckIcon,
    CloudIcon,
    CloudWarningIcon,
    CloudXIcon,
  } from 'phosphor-svelte';
  import MarkdownView from '$components/common/MarkdownView.svelte';
  import {AppInfoCommand} from '$commands';

  let {autoCheck = true}: { autoCheck?: boolean } = $props();

  type UpdaterStatus =
    | 'Idle'
    | 'Checking'
    | 'Available'
    | 'UpToDate'
    | 'Downloading'
    | 'Downloaded'
    | 'Error';

  let status = $state<UpdaterStatus>('Idle');
  let update = $state<Update | null>(null);
  let errorMessage = $state('');
  let installing = $state(false);
  let isVisible = $state(false);
  let currentVersion = $state('');
  let downloadedBytes = $state(0);
  let totalBytes = $state(0);
  let cancelRequested = $state(false);

  // 计算进度百分比
  let percent = $derived(
    totalBytes > 0 ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)) : 0,
  );

  async function checkForUpdates() {
    if (status === 'Checking') return;
    status = 'Checking';
    errorMessage = '';
    try {
      const result = await check();
      if (result) {
        update = result;
        downloadedBytes = 0;
        totalBytes = 0;
        status = 'Available';
      } else {
        status = 'UpToDate';
      }
    } catch (e) {
      status = 'Error';
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  async function download() {
    if (!update || status === 'Downloading') return;
    // 重置取消标志和进度
    cancelRequested = false;
    status = 'Downloading';
    errorMessage = '';
    downloadedBytes = 0;
    totalBytes = 0;

    try {
      await update.download((event) => {
        // 在每次回调中检查是否被取消
        if (cancelRequested) {
          throw new Error('CANCELLED');
        }

        if (event.event === 'Started') {
          totalBytes = event.data.contentLength ?? 0;
        } else if (event.event === 'Progress') {
          downloadedBytes += event.data.chunkLength;
        }
      });

      // 下载完成，检查是否在下载过程中被取消（以防最后一个回调未触发）
      if (cancelRequested) {
        throw new Error('CANCELLED');
      }

      status = 'Downloaded';
    } catch (e) {
      // 处理取消请求
      if (e instanceof Error && e.message === 'CANCELLED') {
        // 静默回退到 Available 状态，保留更新信息
        status = 'Available';
        downloadedBytes = 0;
        totalBytes = 0;
        return;
      }

      // 其他错误
      status = 'Error';
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  /** 取消下载 */
  function cancelDownload() {
    if (status === 'Downloading') {
      cancelRequested = true;
    }
  }

  /** 安装并重启 */
  async function installAndRestart() {
    if (!update || installing) return;
    installing = true;
    try {
      await update.install();
      await relaunch();
    } catch (e) {
      installing = false;
      status = 'Error';
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  function retry() {
    if (status !== 'Error') return;
    // 判断错误来源：如果没有 update 信息，说明是检查阶段错误
    if (!update) {
      void checkForUpdates();
    } else {
      void download();
    }
  }

  /** 打开弹窗 */
  function openPopup() {
    isVisible = true;
  }

  /** 关闭弹窗 */
  function closePopup() {
    if (installing) return; // 安装中不允许关闭
    isVisible = false;
  }

  onMount(async () => {
    // 并行获取当前版本和检查更新
    const tasks: Promise<any>[] = [
      AppInfoCommand.app_info().then((info) => {
        currentVersion = info.version;
      }),
    ];

    if (autoCheck) {
      tasks.push(checkForUpdates());
    }

    await Promise.allSettled(tasks); // 使用 allSettled 避免一个失败影响另一个
  });

  onDestroy(() => {
    // 如果组件卸载时正在下载，自动取消
    if (status === 'Downloading') {
      cancelRequested = true;
    }
  });
</script>

{#if status == 'Idle'}
  <button
    aria-label="检查更新"
    class="icon-button"
    title="检查更新"
    onclick={checkForUpdates}
  >
    <CloudIcon size="16" weight="bold"/>
  </button>
{:else if status == 'Checking'}
  <button
    aria-label="检查更新中"
    class="icon-button"
    title="检查更新中"
  >
    <CircleNotchIcon size="16" style="animation: spin 1.5s linear infinite" weight="bold"/>
  </button>
{:else if status == 'Available'}
  <button
    aria-label="发现新版本"
    class="icon-button primary"
    title="发现新版本"
    onclick={openPopup}
  >
    <CloudWarningIcon size="16" weight="bold"/>
  </button>
{:else if status == 'UpToDate'}
  <button
    aria-label="已是最新版"
    class="icon-button"
    title="已是最新版"
    onclick={openPopup}
  >
    <CloudCheckIcon size="16" weight="bold"/>
  </button>
{:else if status == 'Downloading'}
  <button
    aria-label="正在下载"
    class="icon-button primary"
    title="正在下载"
    onclick={openPopup}
  >
    <CloudArrowDownIcon
      size="16"
      style="animation: pulse 1.5s var(--transition-ease-in-out) infinite, blink 1.5s var(--transition-ease-in-out) infinite"
      weight="bold"
    />
  </button>
{:else if status == 'Downloaded'}
  <button
    aria-label="新版本已就绪，重启以应用更新"
    class="icon-button success"
    title="新版本已就绪，重启以应用更新"
    onclick={openPopup}
  >
    <CloudCheckIcon size="16" weight="bold"/>
  </button>
{:else if status == 'Error'}
  <button
    aria-label="下载失败"
    class="icon-button error"
    title="下载失败"
    onclick={openPopup}
  >
    <CloudXIcon size="16" weight="bold"/>
  </button>
{/if}

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closePopup}>
    <div class="popup" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-title">
        {#if status == 'Idle'}
          检查更新
        {:else if status == 'Checking'}
          正在检查更新，请稍候……
        {:else if status == 'Available'}
          发现新版本 v{update?.version}
        {:else if status == 'UpToDate'}
          已是最新版本
        {:else if status == 'Downloading'}
          正在下载更新
        {:else if status == 'Downloaded'}
          更新已就绪
        {:else if status == 'Error'}
          更新出错
        {:else}
          更新
        {/if}
      </h3>

      {#if status == 'Available'}
        <span class="text-content">发布于：{update?.date}</span>
        <div class="changelog">
          <MarkdownView markdown={update?.body ?? ''}/>
        </div>
      {:else if status == 'Downloading'}
        <div class="progress">
          <div class="progress-bar">
            <div class="progress-fill" style:width="{percent}%"></div>
          </div>
          <span class="progress-text">{percent}%</span>
        </div>
        <p class="text-content" style="margin-top: var(--space-xs);">
          正在下载 {update?.version}，请稍候…
        </p>
      {:else if status == 'Downloaded'}
        <p class="text-content">
          新版本已下载完成，点击「重启并更新」应用更新（应用将自动关闭并重新打开）。
        </p>
      {:else if status == 'Error'}
        <div class="text-content" style="color: var(--color-error)">{errorMessage}</div>
      {:else if status == 'UpToDate'}
        <p class="text-content">当前版本 <b>{currentVersion}</b> 已是最新版本。</p>
      {/if}

      <!-- 按钮组 -->
      <div class="button-group">
        <!-- 取消按钮：下载中显示为“取消下载”，其他状态为“关闭” -->
        <button class="button" onclick={closePopup} disabled={installing}>
          {#if status == 'Downloading'}
            取消下载
          {:else}
            关闭
          {/if}
        </button>

        <!-- 主要操作按钮 -->
        {#if status == 'Available'}
          <button class="button warn" onclick={checkForUpdates}>重新获取更新</button>
          <button class="button yes" onclick={() => void download()}>下载</button>
        {:else if status == 'Downloading'}
          <!-- 下载中，显示取消按钮（已在上面），此处不重复，但也可保留一个禁用状态 -->
          <button class="button yes" disabled>下载中…</button>
        {:else if status == 'Downloaded'}
          <button
            class="button yes"
            onclick={() => void installAndRestart()}
            disabled={installing}
          >
            {installing ? '重启中…' : '重启并更新'}
          </button>
        {:else if status == 'Error'}
          <button class="button warn" onclick={retry}>重试</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .changelog {
    max-height: 16rem;
    overflow-y: auto;
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-card);
    border: var(--border-size-xxs) solid var(--border-color-3);
    border-radius: var(--radius-sm);
  }
</style>
