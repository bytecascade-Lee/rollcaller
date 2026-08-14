<!-- TreeNav.svelte -->
<script lang="ts">
  import metaData from '$resources/help/meta.json';
  import type {NavItem, TreeNode} from "$types";
  import {CaretDownIcon, CaretRightIcon} from "phosphor-svelte";

  function buildNavTree(nodes: Record<string, TreeNode>, order: Record<string, string[]>): NavItem[] {
    const parentToChildren = new Map();

    // 收集所有 parentId，用于检测孤儿节点
    const allParentIds = new Set();

    for (const [id, node] of Object.entries(nodes)) {
      const parentId = node.parentId;

      if (parentId !== null) {
        allParentIds.add(parentId);
      }

      if (!parentToChildren.has(parentId)) {
        parentToChildren.set(parentId, []);
      }
      parentToChildren.get(parentId).push(id);
    }

    // 2. 检测孤儿节点：parentId 指向不存在的节点
    const orphanNodes = [];
    for (const [id, node] of Object.entries(nodes)) {
      if (node.parentId !== null && !nodes[node.parentId]) {
        orphanNodes.push({id, parentId: node.parentId});
      }
    }

    // 3. 递归构建树
    function buildChildren(parentId: string | null): NavItem[] {
      const childIds = parentToChildren.get(parentId) || [];

      // 按 order 排序
      const orderedChildIds = sortChildren(childIds, parentId, order);

      return orderedChildIds
        .filter((id) => nodes[id]) // 过滤掉 nodes 中不存在的 id
        .map((id) => {
          const node = nodes[id];
          const children = buildChildren(id);

          return {
            id,
            title: node.title,
            parentId: node.parentId,
            children,
            isLeaf: children.length === 0,
          };
        });
    }

    // 4. 构建根节点（parentId === null）
    return buildChildren(null);
  }

  function sortChildren(childIds: string[], parentId: string | null, order: Record<string, string[]>): string[] {
    const orderKey = parentId === null ? 'root' : parentId;
    const ordered = order[orderKey] || [];

    // 创建一个 Set 用于快速查找
    const orderSet = new Set(ordered);

    // 1. 先按 order 中的顺序排列
    const inOrder = ordered.filter((id) => childIds.includes(id));

    // 2. 不在 order 中的节点，按字母序排在后面
    const notInOrder = childIds
      .filter((id) => !orderSet.has(id))
      .sort((a, b) => a.localeCompare(b));

    return [...inOrder, ...notInOrder];
  }

  let navTree = $state<NavItem[]>(buildNavTree(metaData.nodes, metaData.order));
  let expandedIds = $state<Set<string>>(new Set());


  function handleNodeClick(item: NavItem) {
    if (item.isLeaf) {
      console.warn(`叶子结点${item.id}被点击`);
    } else {
      console.info(`父节点${item.id}被点击`);
      toggleExpand(item.id);
    }
  }

  function toggleExpand(id: string) {
    const newSet = new Set(expandedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    expandedIds = newSet;
  }

  function renderItems(items: NavItem[], level: number = 0): any {
    return items.map((item) => {
      const hasChildren = !item.isLeaf;
      const isExpanded = expandedIds.has(item.id);

      return {
        item,
        hasChildren,
        isExpanded,
        level,
        children: hasChildren ? renderItems(item.children, level + 1) : [],
      };
    });
  }

  let rendered: ReturnType<typeof renderItems> = $derived(renderItems(navTree));
</script>

<div class="tree-nav">
  {#each rendered as node (node.item.id)}
    <!-- 使用 {@render} 调用 snippet，并传递参数 -->
    {@render treeItem(node, handleNodeClick)}
  {/each}
</div>

{#snippet treeItem(node, onClick)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="tree-node" style:padding-left={`${node.level * 20}px`}>
    <!-- 节点内容 -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tree-node-content"
      class:is-leaf={node.item.isLeaf}
      class:is-parent={!node.item.isLeaf}
      class:is-expanded={node.isExpanded}
      onclick={() => onClick(node.item)}
    >
      {#if node.hasChildren}
        <span class="arrow">
          {#if !node.isExpanded}
            <CaretRightIcon size="14"/>
          {:else}
            <CaretDownIcon size="14"/>
          {/if}
        </span>
      {:else}
        <span class="arrow-placeholder"></span>
      {/if}

      <span class="title">{node.item.title}</span>

      <span class="badge">
        {node.item.isLeaf ? '📄' : '📁'}
      </span>
    </div>

    {#if node.hasChildren && node.isExpanded}
      <div class="tree-children">
        {#each node.children as child (child.item.id)}
          {@render treeItem(child, onClick)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

