# ID: src/wrap-text.ts:7
const memoizedWraps: Record<string, string> = {};

const wrapTextToWidth = (
	content: string,
	columnLimit: number,
	mode: Styles['textWrap'],
): string => {
	const memoKey = content + String(columnLimit) + String(mode);
	const previous = memoizedWraps[memoKey];

	if (previous) {
		return previous;
	}

	let result = content;

	if (mode === 'wrap') {
		result = wrapAnsi(content, columnLimit, {
			trim: false,
			hard: true,
		});
	}

	if (mode === 'hard') {
		result = wrapAnsi(content, columnLimit, {
			trim: false,
			hard: true,
			wordWrap: false,
		});
	}

	if (mode!.startsWith('truncate')) {
		let side: 'end' | 'middle' | 'start' = 'end';

		if (mode === 'truncate-middle') {
			side = 'middle';
		}

		if (mode === 'truncate-start') {
			side = 'start';
		}

		result = cliTruncate(content, columnLimit, {position: side});
	}

	memoizedWraps[memoKey] = result;

	return result;
};
