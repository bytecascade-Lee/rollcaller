<script lang="ts">
  import type {WebviewWindow} from "@tauri-apps/api/webviewWindow";
  import {openUrl} from "@tauri-apps/plugin-opener";
  import {
    ArrowLeftIcon,
    ArrowRightIcon,
    CopyIcon,
    DiceFourIcon,
    GithubLogoIcon,
    MagnifyingGlassIcon,
    MinusIcon,
    SquareIcon,
    XIcon
  } from "phosphor-svelte";
  import {onMount} from "svelte";
  import "$styles/logo.css";
  import "$styles/titlebar.css";
  import "$styles/icon-button.css";
  import logo from "$assets/icon.ico";
  import {WindowsCommand} from "$commands";
  import SearchHelpDocs from "$components/help/SearchHelpDocs.svelte";
  import type {EntryRegistry} from "$routing/registry";
  import {HelpOverlay, type HelpOverlayName} from "$types";
  import {invoke} from "@tauri-apps/api/core";

  let {window, overlays, onback, onforward, canGoBack, canGoForward}: {
    window: WebviewWindow,
    /** 本窗口的叠加层注册表（注册表归窗口所有，逐个传下去而不是全局单例） */
    overlays: EntryRegistry<HelpOverlayName>,
    onback?: () => void,
    onforward?: () => void,
    canGoBack?: boolean,
    canGoForward?: boolean
  } = $props();
  let isMaximized = $state(false);
  let snapTimer: ReturnType<typeof setTimeout> | undefined;
  let anchor = $state<HTMLElement | null>(null);

  onMount(() => {
    const refresh = () => window.isMaximized().then((v) => (isMaximized = v));
    refresh();
    const unlisten = window.onResized(refresh);
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  /** 悬停最大化按钮 620ms 后弹出 Windows 11 贴靠布局（Snap Layout） */
  function onMaximizeHover() {
    if (isMaximized) return;
    snapTimer = setTimeout(() => {
      window.setFocus().then(() => invoke("plugin:decorum|show_snap_overlay"));
    }, 620);
  }

  function onMaximizeLeave() {
    clearTimeout(snapTimer);
  }
</script>

<div
  class="titlebar"
  data-tauri-decorum-tb
>
  <div
    aria-label="Rollcaller"
    class="logo"
    title="Rollcaller"
  >
    <img alt="logo" src={logo}>
  </div>
  <div
    class="titlebar-drag-region"
    data-tauri-drag-region="deep"
  >
    <div
      class="text-subtitle"
      style:padding-top="6px"
    >
      自动点名 - 帮助文档
    </div>
  </div>
  <div
    class="icon-button-group"
    style:padding-right="12px"
  >
    <button
      aria-label="返回"
      class="icon-button"
      disabled={!canGoBack}
      onclick={onback}
      title="返回"
    >
      <ArrowLeftIcon size="16" weight="bold"/>
    </button>
    <button
      aria-label="前进"
      class="icon-button"
      disabled={!canGoForward}
      onclick={onforward}
      title="前进"
    >
      <ArrowRightIcon size="16" weight="bold"/>
    </button>
    <button
      aria-label="搜索"
      class="icon-button"
      onclick={(e) => {
        anchor = e.currentTarget;
        overlays.open(HelpOverlay.search);
      }}
      title="搜索"
    >
      <MagnifyingGlassIcon size="16" weight="bold"/>
    </button>
    <button
      aria-label="主界面"
      class="icon-button"
      onclick={WindowsCommand.openAppWindow}
      title="主界面"
    >
      <DiceFourIcon size="16" weight="bold"/>
    </button>
    <button
      aria-label="项目主页"
      class="icon-button"
      onclick={() => openUrl("https://github.com/bytecascade-Lee/rollcaller")}
      title="项目主页"
    >
      <GithubLogoIcon size="16" weight="fill"/>
    </button>
    <button
      aria-label="最小化"
      class="icon-button"
      onclick={() => window.minimize()}
      title="最小化"
    >
      <MinusIcon size="16" weight="bold"/>
    </button>
    <button
      aria-label={isMaximized ? "还原" : "最大化"}
      class="icon-button"
      onclick={() => window.toggleMaximize()}
      onmouseenter={onMaximizeHover}
      onmouseleave={onMaximizeLeave}
      title={isMaximized ? "还原" : "最大化"}
    >
      {#if isMaximized}
        <CopyIcon size="16" weight="bold"/>
      {:else}
        <SquareIcon size="16" weight="bold"/>
      {/if}
    </button>
    <button
      aria-label="关闭"
      class="icon-button danger"
      onclick={() => window.close()}
      title="关闭"
    >
      <XIcon size="16" weight="bold"/>
    </button>
  </div>
</div>

<SearchHelpDocs bind:anchor={anchor} {overlays}/>
