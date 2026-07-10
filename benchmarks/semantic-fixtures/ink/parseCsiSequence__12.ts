# ID: src/input-parser.ts:32
const readCsiSequenceFrom = (
	input: string,
	startIndex: number,
	prefixLength: number,
): ParsedSequence => {
	const payloadStart = startIndex + prefixLength + 1;
	let cursor = payloadStart;

	for (; cursor < input.length; cursor++) {
		const byte = input.codePointAt(cursor);
		if (byte === undefined) {
			return 'pending';
		}

		if (isCsiParameterByte(byte) || isCsiIntermediateByte(byte)) {
			continue;
		}

		// Keep legacy function-key sequences like ESC[[A and ESC[[5~ intact.
		if (byte === 0x5b && cursor === payloadStart) {
			continue;
		}

		if (isCsiFinalByte(byte)) {
			return {
				sequence: input.slice(startIndex, cursor + 1),
				nextIndex: cursor + 1,
			};
		}

		return undefined;
	}

	return 'pending';
};
