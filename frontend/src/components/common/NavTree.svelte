<script lang="ts">
  import type {NavItem, TreeNode} from "$types";
  import {buildNavTree} from "$utils/navTree";
  import {CaretDownIcon, CaretRightIcon} from "phosphor-svelte";

  let {
    nodes,
    order,
    onselect,
    activeId = $bindable(null),
    jumpToken = 0,
    defaultExpanded = []
  }: {
    nodes: Record<string, TreeNode>;
    order: Record<string, string[]>;
    onselect?: (id: string) => void;
    activeId?: string | null;
    jumpToken?: number;
    defaultExpanded?: string[];
  } = $props();

  const navTree = $derived(buildNavTree(nodes, order));
  let expandedIds = $derived<Set<string>>(new Set(defaultExpanded));

  function toggleExpand(id: string) {
    const next = new Set(expandedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expandedIds = next;
  }

  /** 返回从根到目标节点的祖先链（不含目标自身），供选中时自动展开。 */
  function collectAncestors(items: NavItem[], target: string, trail: string[] = []): string[] | null {
    for (const item of items) {
      if (item.id === target) return trail;
      const found = collectAncestors(item.children, target, [...trail, item.id]);
      if (found) return found;
    }
    return null;
  }

  function handleSelect(item: NavItem) {
    activeId = item.id;
    const ancestors = collectAncestors(navTree, item.id);
    if (ancestors && ancestors.length) {
      const next = new Set(expandedIds);
      ancestors.forEach((id) => next.add(id));
      expandedIds = next;
    }
    onselect?.(item.id);
  }

  // 外部跳转（如内部链接）：折叠全部，仅展开到当前 activeId 的路径。
  // 用 lastJumpToken 做守卫，避免用户手动点击树（activeId 变化）触发折叠。
  let lastJumpToken = 0;
  $effect(() => {
    if (jumpToken !== lastJumpToken) {
      lastJumpToken = jumpToken;
      if (jumpToken > 0 && activeId) {
        const ancestors = collectAncestors(navTree, activeId);
        const next = new Set<string>();
        ancestors?.forEach((id) => next.add(id));
        expandedIds = next;
      }
    }
  });
</script>

<div class="tree-nav">
  {#each navTree as item (item.id)}
    {@render treeItem(item, 0)}
  {/each}
</div>

{#snippet treeItem(item: NavItem, level: number)}
  {@const hasChildren = item.children.length > 0}
  {@const isExpanded = expandedIds.has(item.id)}
  {@const isActive = activeId === item.id}
  <div class="tree-node">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="nav-item"
      class:is-active={isActive}
      class:is-leaf={!hasChildren}
      class:is-parent={hasChildren}
      class:is-expanded={isExpanded}
      style:padding-left={`${level * 8}px`}
      onclick={() => (hasChildren ? toggleExpand(item.id) : handleSelect(item))}
    >
      {#if hasChildren}
        <span class="arrow">
          {#if isExpanded}
            <CaretDownIcon size="12"/>
          {:else}
            <CaretRightIcon size="12"/>
          {/if}
        </span>
      {:else}
        <span class="arrow-placeholder"></span>
      {/if}

      <span class="title">{item.title}</span>
    </div>

    {#if hasChildren && isExpanded}
      <div class="tree-children">
        {#each item.children as child (child.id)}
          {@render treeItem(child, level + 1)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<style>
  .arrow,
  .arrow-placeholder {
    width: 12px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .title {
    flex: 1;
    font-size: var(--font-size-xs);
  }
</style>
