# ID: src/dom.ts:111
export const attachChildNode = (
	parent: DOMElement,
	child: DOMElement,
): void => {
	if (child.parentNode) {
		removeChildNode(child.parentNode, child);
	}

	child.parentNode = parent;
	parent.childNodes.push(child);

	if (child.yogaNode) {
		parent.yogaNode?.insertChild(
			child.yogaNode,
			parent.yogaNode.getChildCount(),
		);
	}

	if (
		parent.nodeName === 'ink-text' ||
		parent.nodeName === 'ink-virtual-text'
	) {
		markNodeAsDirty(parent);
	}
};
