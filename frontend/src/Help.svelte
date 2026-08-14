<script lang="ts">
  import "highlight.js/styles/github.css";
  import {onMount} from "svelte";
  import NavTree from "$components/common/NavTree.svelte";
  import MarkdownView from "$components/common/MarkdownView.svelte";
  import {helpStore} from "$stores/helpStore.svelte";
  import metaData from "$resources/help/meta.json";

  let activeId = $state<string | null>(null);

  onMount(() => {
    helpStore.load("readme");
  });
</script>

<div class="app">
  <aside class="sidebar">
    <NavTree
      nodes={metaData.nodes}
      order={metaData.order}
      bind:activeId={activeId}
      onselect={(id) => helpStore.load(id)}
    />
  </aside>
  <main class="content">
    <MarkdownView markdown={helpStore.content}/>
  </main>
</div>

<style>
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    width: 240px;
    flex-shrink: 0;
    overflow-y: auto;
    padding: var(--space-xs) var(--space-xxs);
    background: var(--color-page);
    border-right: var(--border-size-xxs) solid var(--border-color-3);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    background: var(--color-card);
  }
</style>
