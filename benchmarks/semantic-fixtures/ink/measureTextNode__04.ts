# ID: src/dom.ts:219
const measureInkTextNode = function (
	node: DOMNode,
	availableWidth: number,
): {width: number; height: number} {
	const content =
		node.nodeName === '#text' ? node.nodeValue : squashTextNodes(node);

	const measured = measureText(content);

	// Text already fits the container — no wrapping required.
	if (measured.width <= availableWidth) {
		return measured;
	}

	// Yoga is probing whether the node fits into sub-1px space while shrinking;
	// answer "no" by returning the natural dimensions.
	if (measured.width >= 1 && availableWidth > 0 && availableWidth < 1) {
		return measured;
	}

	const wrapMode = node.style?.textWrap ?? 'wrap';
	const wrapped = wrapText(content, availableWidth, wrapMode);

	return measureText(wrapped);
};
