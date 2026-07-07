# ID: src/squash-text-nodes.ts:10
const flattenTextNodes = (element: DOMElement): string => {
	let combined = '';

	for (let position = 0; position < element.childNodes.length; position++) {
		const child = element.childNodes[position];

		if (child === undefined) {
			continue;
		}

		let piece = '';

		if (child.nodeName === '#text') {
			piece = child.nodeValue;
		} else {
			if (
				child.nodeName === 'ink-text' ||
				child.nodeName === 'ink-virtual-text'
			) {
				piece = flattenTextNodes(child);
			}

			// Concatenated text nodes bypass Output's transform pass, so run each
			// child's transform manually before joining.
			if (
				piece.length > 0 &&
				typeof child.internal_transform === 'function'
			) {
				piece = child.internal_transform(piece, position);
			}
		}

		combined += piece;
	}

	return sanitizeAnsi(combined);
};
