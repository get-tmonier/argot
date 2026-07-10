# ID: src/render-background.ts:5
const paintBackground = (
	x: number,
	y: number,
	node: DOMNode,
	output: Output,
): void => {
	if (!node.style.backgroundColor) {
		return;
	}

	const boxWidth = node.yogaNode!.getComputedWidth();
	const boxHeight = node.yogaNode!.getComputedHeight();

	// Inset the fill area so it never overlaps borders.
	const hasBorder = node.style.borderStyle;
	const leftInset = hasBorder && node.style.borderLeft !== false ? 1 : 0;
	const rightInset = hasBorder && node.style.borderRight !== false ? 1 : 0;
	const topInset = hasBorder && node.style.borderTop !== false ? 1 : 0;
	const bottomInset = hasBorder && node.style.borderBottom !== false ? 1 : 0;

	const fillWidth = boxWidth - leftInset - rightInset;
	const fillHeight = boxHeight - topInset - bottomInset;

	if (!(fillWidth > 0 && fillHeight > 0)) {
		return;
	}

	const filledRow = colorize(
		' '.repeat(fillWidth),
		node.style.backgroundColor,
		'background',
	);

	for (let row = 0; row < fillHeight; row++) {
		output.write(x + leftInset, y + topInset + row, filledRow, {
			transformers: [],
		});
	}
};
