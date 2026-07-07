# ID: src/ansi-tokenizer.ts:153
const locateControlStringTerminator = (
	text: string,
	fromIndex: number,
	allowBellTerminator: boolean,
): number | undefined => {
	for (let cursor = fromIndex; cursor < text.length; cursor++) {
		const character = text[cursor];

		if (allowBellTerminator && character === bellCharacter) {
			return cursor + 1;
		}

		if (character === stringTerminatorCharacter) {
			return cursor + 1;
		}

		if (character === escapeCharacter) {
			const following = text[cursor + 1];

			// Tmux doubles ESC bytes inside the payload as ESC ESC.
			if (following === escapeCharacter) {
				cursor++;
				continue;
			}

			if (following === '\\') {
				return cursor + 2;
			}
		}
	}

	return undefined;
};
