# ID: src/render-node-to-output.ts:32
export const buildScreenReaderText = (
	node: DOMElement,
	options: {
		parentRole?: string;
		skipStaticElements?: boolean;
	} = {},
): string => {
	if (options.skipStaticElements && node.internal_static) {
		return '';
	}

	if (node.yogaNode?.getDisplay() === Yoga.DISPLAY_NONE) {
		return '';
	}

	let text = '';

	if (node.nodeName === 'ink-text') {
		text = squashTextNodes(node);
	} else if (node.nodeName === 'ink-box' || node.nodeName === 'ink-root') {
		const isRow =
			node.style.flexDirection === 'row' ||
			node.style.flexDirection === 'row-reverse';
		const separator = isRow ? ' ' : '\n';

		const isReversed =
			node.style.flexDirection === 'row-reverse' ||
			node.style.flexDirection === 'column-reverse';
		const orderedChildren = isReversed
			? [...node.childNodes].reverse()
			: [...node.childNodes];

		text = orderedChildren
			.map(child =>
				buildScreenReaderText(child as DOMElement, {
					parentRole: node.internal_accessibility?.role,
					skipStaticElements: options.skipStaticElements,
				}),
			)
			.filter(Boolean)
			.join(separator);
	}

	if (node.internal_accessibility) {
		const {role, state} = node.internal_accessibility;

		if (state) {
			const stateKeys = Object.keys(state) as Array<keyof typeof state>;
			const activeStates = stateKeys.filter(key => state[key]).join(', ');

			if (activeStates) {
				text = `(${activeStates}) ${text}`;
			}
		}

		if (role && role !== options.parentRole) {
			text = `${role}: ${text}`;
		}
	}

	return text;
};
