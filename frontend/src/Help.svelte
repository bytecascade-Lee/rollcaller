<script lang="ts">
  import "highlight.js/styles/github.css";
  import "$styles/global.css";
  import "$styles/tokens.css";
  import "$styles/sidebar.css";
  import "$styles/nav.css";
  import "$styles/content.css";
  import "$styles/footbar.css";
  import "$styles/text.css";
  import {onMount} from "svelte";
  import NavTree from "$components/common/NavTree.svelte";
  import MarkdownView from "$components/common/MarkdownView.svelte";
  import {helpStore} from "$stores/helpStore.svelte";
  import {AppInfoCommand} from "$commands";
  import type {AppInfo} from "$types";
  import {GearIcon} from "phosphor-svelte";
  import metaData from "$resources/help/meta.json";

  let activeId = $state<string | null>(null);
  let APP_INFO = $state<AppInfo>({
    branch: "",
    commit_count: "",
    short_hash: "",
    commit_time: "",
    version: "",
    build_time: ""
  });

  onMount(async () => {
    APP_INFO = await AppInfoCommand.app_info();
    helpStore.load("readme");
  });
</script>

<div class="shell">
  <aside class="sidebar">
    <nav class="nav">
      <NavTree
        nodes={metaData.nodes}
        order={metaData.order}
        bind:activeId={activeId}
        onselect={(id) => helpStore.load(id)}
      />
    </nav>
  </aside>

  <main class="content">
    {#if helpStore.content}
      <div class="active">
        <MarkdownView markdown={helpStore.content}/>
      </div>
    {:else}
      <div class="empty active">
        <p class="text-content">当前节点下暂无内容</p>
      </div>
    {/if}
  </main>

  <footer class="footbar">
    <div>
      <GearIcon size="14" weight="bold" style="display: none"/>
      {APP_INFO.version}+{APP_INFO.branch}.{APP_INFO.commit_count}.{APP_INFO.short_hash}#{APP_INFO.commit_time}
      #{APP_INFO.build_time}
    </div>
  </footer>
</div>

<style>
  .shell {
    display: grid;
    grid-auto-columns: 150px 1fr;
    grid-template-rows: 1fr auto;
    grid-template-areas:
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
