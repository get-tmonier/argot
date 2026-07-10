# ID: src/measure-text.ts:10
const computeTextDimensions = (text: string): Output => {
	if (text.length === 0) {
		return {
			width: 0,
			height: 0,
		};
	}

	const cached = cache.get(text);

	if (cached) {
		return cached;
	}

	const width = widestLine(text);
	const height = text.split('\n').length;
	const dimensions = {width, height};
	cache.set(text, dimensions);

	return dimensions;
};
