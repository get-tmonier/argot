# ID: src/render-node-to-output.ts:18
const offsetTextByPadding = (node: DOMElement, text: string): string => {
	const firstChildYoga = node.childNodes[0]?.yogaNode;

	if (firstChildYoga) {
		const offsetX = firstChildYoga.getComputedLeft();
		const offsetY = firstChildYoga.getComputedTop();
		text = '\n'.repeat(offsetY) + indentString(text, offsetX);
	}

	return text;
};
