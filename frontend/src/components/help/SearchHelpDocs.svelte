<script lang="ts">
  import {overlayController} from "$controllers/popupController";
  import {clickOutside} from "$actions";
  import {type HelpSearchResult, helpStore} from "$stores/helpStore.svelte";
  import {MagnifyingGlassIcon} from "phosphor-svelte";

  let {anchor = $bindable()} = $props<{ anchor: HTMLElement | null }>();

  let isVisible = $state(false);
  let query = $state("");
  let searchInput: HTMLInputElement | undefined = $state();
  let results = $state<HelpSearchResult[]>([]);
  let selectedIndex = $state(0);

  // 输入变化即搜索；索引构建完成（indexing 翻转）后自动补搜一次
  $effect(() => {
    helpStore.indexing;
    results = helpStore.search(query);
    selectedIndex = 0;
  });

  // 打开时聚焦输入框
  $effect(() => {
    if (isVisible) {
      requestAnimationFrame(() => searchInput?.focus());
    }
  });

  /** 选中结果：跳转到对应文档并关闭弹层。 */
  function select(id: string) {
    close();
    if (helpStore.navigate) {
      helpStore.navigate(id);
    } else {
      helpStore.load(id);
    }
  }

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  /** 片段已由 store 清理为纯文本（符号/链接 URL 已移除），这里只做 HTML 转义 + 关键词高亮。 */
  function renderSnippet(text: string, query: string): string {
    const html = escapeHtml(text);
    const q = query.trim();
    if (!q) return html;
    const esc = escapeHtml(q).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return html.replace(new RegExp(esc, "gi"), '<strong class="kw">$&</strong>');
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      close();
      return;
    }
    if (results.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % results.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + results.length) % results.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      select(results[selectedIndex].id);
    }
  }

  export function open() {
    isVisible = true;
    query = "";
    selectedIndex = 0;
    helpStore.ensureIndexed();
  }

  export function close() {
    isVisible = false;
  }

  $effect(() => {
    overlayController.register("HelpSearch", {
      open,
      close,
      isVisible: () => isVisible,
    });
    return () => overlayController.unregister("HelpSearch");
  });
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="popup help"
    use:clickOutside={{ callback: close, exclude: anchor ?? undefined }}
    onkeydown={handleKeydown}
    onmousedown={() => searchInput?.focus()}
  >
    <div
      class="search help"
    >
      <MagnifyingGlassIcon size="18"/>
      <input
        class="help"
        type="search"
        placeholder="搜索帮助文档..."
        bind:value={query}
        bind:this={searchInput}
      />
    </div>
    <div class="search-results">
      {#if helpStore.indexing}
        <div class="state">加载索引中...</div>
      {:else if results.length === 0 && query.trim()}
        <div class="state">未找到匹配内容</div>
      {:else}
        {#each results as result, i (result.key)}
          <button
            class="search-result-item"
            class:selected={i === selectedIndex}
            onmouseenter={() => (selectedIndex = i)}
            onclick={() => select(result.id)}
          >
            <span class="result-title">{@html renderSnippet(result.title, query)}</span>
            {#if result.snippet}
              <span class="result-snippet">{@html renderSnippet(result.snippet, query)}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </div>
{/if}

<style>
  /* 覆盖 popup.css 的 .popup.help（var(--size-14) 太窄且 left:25% 不对齐）：
     固定宽度、顶部居中、不受内容影响 */
  .popup.help {
    position: fixed;
    top: 38px;
    left: 50%;
    width: 480px;
    max-width: 480px;
    transform: translateX(-50%);
  }

  /* 搜索框占满 popup 内容宽度，视觉沿用 search.css 的 .search */
  .search.help {
    width: 100%;
  }

  /* 覆盖 search.css 的 .search.help input { width: inherit }：
     输入框弹性占满剩余空间，避免被内容撑开 */
  .search.help input {
    flex: 1;
    min-width: 0;
    width: auto;
  }

  .search-results {
    display: flex;
    flex-direction: column;
    gap: var(--space-xxs);
    overflow-y: auto;
    /* 可视约 8 条，其余滚动查看 */
    max-height: 240px;
  }

  /* 关键词高亮：加粗 + 主题色（{@html} 注入的节点不受 scoped 约束，需 :global） */
  :global(.kw) {
    color: var(--color-primary);
    font-weight: var(--font-weight-bold);
  }

  .search-result-item {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-sm);
    width: 100%;
    padding: var(--space-xxs) var(--space-xs);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-color-content);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-sm);
    text-align: left;
    cursor: pointer;
    transition: background-color var(--transition-duration-md) var(--transition-ease);
  }

  .search-result-item:hover,
  .search-result-item.selected {
    background: var(--color-hover);
  }

  .result-title {
    flex-shrink: 0;
    color: var(--text-color-primary);
    font-weight: var(--font-weight-medium);
  }

  .result-snippet {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--text-color-secondary);
    font-size: var(--font-size-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
