# ID: src/sanitize-ansi.ts:9
const stripLayoutBreakingAnsi = (text: string): string => {
	if (!hasAnsiControlCharacters(text)) {
		return text;
	}

	let result = '';

	for (const token of tokenizeAnsi(text)) {
		if (token.type === 'text' || token.type === 'osc') {
			result += token.value;
			continue;
		}

		// Keep only plain SGR sequences (colors, bold, etc.); drop cursor moves,
		// screen clears, and other layout-breaking control sequences.
		if (
			token.type === 'csi' &&
			token.finalCharacter === 'm' &&
			token.intermediateString === '' &&
			sgrParametersRegex.test(token.parameterString)
		) {
			result += token.value;
		}
	}

	return result;
};
