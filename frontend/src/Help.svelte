<script lang="ts">
  import "open-props/style"
  import "highlight.js/styles/github.css";
  import "$styles/tokens.css";
  import "$styles/sidebar.css";
  import "$styles/nav.css";
  import "$styles/content.css";
  import "$styles/footbar.css";
  import "$styles/text.css";
  import {onMount} from "svelte";
  import TitleBar from "$components/common/TitleBar.svelte";
  import {getCurrentWebviewWindow} from "@tauri-apps/api/webviewWindow";
  import NavTree from "$components/common/NavTree.svelte";
  import MarkdownView from "$components/common/MarkdownView.svelte";
  import {helpStore} from "$stores/helpStore.svelte";
  import {AppInfoCommand} from "$commands";
  import type {AppInfo, TreeNode} from "$types";
  import {GearIcon} from "phosphor-svelte";
  import metaData from "$resources/help/meta.json";

  const nodes = metaData.nodes as
    {
      [K in keyof typeof metaData.nodes]: TreeNode & { id: K }
    };
  const window = getCurrentWebviewWindow();
  let activeId = $state("overview");
  let jumpToken = $state(0); // 外部跳转触发信号：自增时 NavTree 折叠到单级
  let scroller: HTMLDivElement | undefined = $state();
  let APP_INFO = $state<AppInfo>({
    branch: "",
    commit_count: "",
    short_hash: "",
    commit_time: "",
    version: "",
    build_time: ""
  });

  /** 内部链接跳转：加载文档，并让左侧树跳转到对应节点（折叠到单级） */
  function handleNavigate(id: string) {
    activeId = id;
    jumpToken += 1;
    helpStore.load(id);
  }

  // 切换文档后滚动条回到顶部
  $effect(() => {
    helpStore.content;
    scroller?.scrollTo(0, 0);
  });

  onMount(async () => {
    APP_INFO = await AppInfoCommand.app_info();
    await helpStore.load(activeId);
  });
</script>

<div class="shell">
  <div class="titlebar-slot">
    <TitleBar window={window} title="自动点名应用 - 帮助文档" label={window.label}/>
  </div>
  <aside class="sidebar">
    <nav class="nav">
      <NavTree
        bind:activeId={activeId}
        jumpToken={jumpToken}
        nodes={nodes}
        onselect={(id) => helpStore.load(id)}
        order={metaData.order}
      />
    </nav>
  </aside>

  <main class="content">
    {#if helpStore.content}
      <div class="active" bind:this={scroller}>
        <MarkdownView markdown={helpStore.content} onnavigate={handleNavigate}/>
      </div>
    {:else}
      <div class="empty active">
        <p class="text-content">当前节点下暂无内容</p>
      </div>
    {/if}
  </main>

  <footer class="footbar">
    <div>
      <GearIcon size="14" style="display: none" weight="bold"/>
      {APP_INFO.version}+{APP_INFO.branch}.{APP_INFO.commit_count}.{APP_INFO.short_hash}#{APP_INFO.commit_time}
      #{APP_INFO.build_time}
    </div>
  </footer>
</div>

<style>
  .shell {
    display: grid;
    grid-auto-columns: 130px 1fr;
    grid-template-rows: auto 1fr auto;
    grid-template-areas:
      "titlebar titlebar"
      "sidebar content"
      "footbar footbar";
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--color-page);
    color: var(--text-color-content);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-sm);
  }

  .titlebar-slot {
    grid-area: titlebar;
  }

  .sidebar {
    min-height: 0;
    overflow: auto;
  }

  .empty {
    align-items: center;
    justify-content: center;
    color: var(--text-color-secondary);
  }

  .content > .active {
    overflow-y: auto;
  }
</style>
