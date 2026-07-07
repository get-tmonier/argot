# ID: src/input-parser.ts:180
const emitBackspaceSplitEvents = (
	text: string,
	events: InputEvent[],
): void => {
	let segmentStart = 0;

	for (let cursor = 0; cursor < text.length; cursor++) {
		const character = text[cursor]!;

		// Split held-backspace bursts (0x7F / 0x08) into individual events so
		// parseKeypress can recognize each one.
		if (character === '\u007F' || character === '\u0008') {
			if (cursor > segmentStart) {
				events.push(text.slice(segmentStart, cursor));
			}

			events.push(character);
			segmentStart = cursor + 1;
		}
	}

	if (segmentStart < text.length) {
		events.push(text.slice(segmentStart));
	}
};
