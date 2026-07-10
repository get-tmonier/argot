# ID: shared/utils/tree.ts:18
export const lineageOf = (node: NavigationNode | null) => {
  const chain: NavigationNode[] = [];
  let current = node;
  while (current && current.parent !== null) {
    const parent = current.parent as NavigationNode;
    chain.unshift(parent);
    current = parent;
  }
  return chain;
};
