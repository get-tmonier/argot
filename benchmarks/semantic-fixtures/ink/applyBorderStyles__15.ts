# ID: src/styles.ts:729
const applyBorderLayout = (
	node: YogaNode,
	style: Styles,
	currentStyle: Styles,
): void => {
	const borderTouched =
		'borderStyle' in style ||
		'borderTop' in style ||
		'borderBottom' in style ||
		'borderLeft' in style ||
		'borderRight' in style;

	if (!borderTouched) {
		return;
	}

	const edgeWidth = currentStyle.borderStyle ? 1 : 0;

	node.setBorder(
		Yoga.EDGE_TOP,
		currentStyle.borderTop === false ? 0 : edgeWidth,
	);
	node.setBorder(
		Yoga.EDGE_BOTTOM,
		currentStyle.borderBottom === false ? 0 : edgeWidth,
	);
	node.setBorder(
		Yoga.EDGE_LEFT,
		currentStyle.borderLeft === false ? 0 : edgeWidth,
	);
	node.setBorder(
		Yoga.EDGE_RIGHT,
		currentStyle.borderRight === false ? 0 : edgeWidth,
	);
};
