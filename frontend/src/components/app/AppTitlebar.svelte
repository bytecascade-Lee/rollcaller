<script lang="ts">
  import {invoke} from "@tauri-apps/api/core";
  import type {WebviewWindow} from "@tauri-apps/api/webviewWindow";
  import {CopyIcon, GithubLogoIcon, MinusIcon, PushPinIcon, SquareIcon, XIcon} from "phosphor-svelte";
  import {onMount} from "svelte";
  import "$styles/logo.css";
  import "$styles/titlebar.css";
  import "$styles/icon-button.css";
  import logo from "$assets/icon.ico";
  import {openUrl} from "@tauri-apps/plugin-opener";

  let {window}: { window: WebviewWindow } = $props();
  let isMaximized = $state(false);
  let snapTimer: ReturnType<typeof setTimeout> | undefined;
  let isAlwaysOnTop = $state(false);

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
      自动点名
    </div>
  </div>
  <div
    class="icon-button-group"
    style:padding-right="12px"
  >
    <button
      class="icon-button"
      title="置顶"
      aria-label="置顶"
      style:background={isAlwaysOnTop ? "var(--color-primary)" : "var(--color-page)"}
      onclick={async () => {
        isAlwaysOnTop = !isAlwaysOnTop;
        await window.setAlwaysOnTop(isAlwaysOnTop);
      }}
    >
      {#if isAlwaysOnTop}
        <PushPinIcon size="16" weight="bold" color="var(--color-page)"/>
      {:else}
        <PushPinIcon size="16" weight="bold"/>
      {/if}
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
