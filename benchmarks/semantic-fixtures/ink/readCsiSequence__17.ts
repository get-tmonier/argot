# ID: src/ansi-tokenizer.ts:102
const scanCsiSequence = (
	text: string,
	fromIndex: number,
):
	| {
			readonly endIndex: number;
			readonly parameterString: string;
			readonly intermediateString: string;
			readonly finalCharacter: string;
	  }
	| undefined => {
	let cursor = fromIndex;

	while (cursor < text.length) {
		if (!isCsiParameterCharacter(text[cursor]!)) {
			break;
		}

		cursor++;
	}

	const parameterString = text.slice(fromIndex, cursor);
	const intermediateStart = cursor;

	while (cursor < text.length) {
		if (!isCsiIntermediateCharacter(text[cursor]!)) {
			break;
		}

		cursor++;
	}

	const intermediateString = text.slice(intermediateStart, cursor);
	const finalCharacter = text[cursor];

	if (finalCharacter === undefined || !isCsiFinalCharacter(finalCharacter)) {
		return undefined;
	}

	return {
		endIndex: cursor + 1,
		parameterString,
		intermediateString,
		finalCharacter,
	};
};
