# ID: shared/utils/tree.ts:29
export const subtreeNodes = (node: NavigationNode, depth = 0) => {
  const below = flattenTree(node).slice(1);
  if (depth === 0) {
    return below;
  }
  const maxDepth = (node.depth as number) + depth;
  return below.filter((d) => (d.depth as number) <= maxDepth);
};
