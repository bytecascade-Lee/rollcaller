import type {NavItem, TreeNode} from "$types";

/**
 * 由 meta 的 nodes/order 构建有序导航树。
 * 纯函数：无组件状态、无副作用，可独立单测。
 */
export function buildNavTree(
  nodes: Record<string, TreeNode>,
  order: Record<string, string[]>
): NavItem[] {
  const childrenOf = new Map<string | null, string[]>();

  for (const [id, node] of Object.entries(nodes)) {
    const parentId = node.parentId ?? null;
    const siblings = childrenOf.get(parentId);
    if (siblings) siblings.push(id);
    else childrenOf.set(parentId, [id]);
  }

  function build(parentId: string | null): NavItem[] {
    const ordered = sortChildren(childrenOf.get(parentId) ?? [], parentId, order);
    return ordered
      .filter((id) => nodes[id]) // 忽略 order 中引用但 nodes 不存在的 id
      .map((id) => {
        const node = nodes[id];
        return {id, title: node.title, parentId: node.parentId, children: build(id)};
      });
  }

  return build(null);
}

/** 先按 order 排列，未列入 order 的子节点按字母序排在末尾。 */
function sortChildren(
  childIds: string[],
  parentId: string | null,
  order: Record<string, string[]>
): string[] {
  const ordered = order[parentId ?? "root"] ?? [];
  const orderedSet = new Set(ordered);

  const inOrder = ordered.filter((id) => childIds.includes(id));
  const notInOrder = childIds
    .filter((id) => !orderedSet.has(id))
    .sort((a, b) => a.localeCompare(b));

  return [...inOrder, ...notInOrder];
}
