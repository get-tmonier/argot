# ID: src/cursor-helpers.ts:45
export const composeReturnToBottom = (
	previousLineCount: number,
	previousCursorPosition: CursorPosition | undefined,
): string => {
	if (!previousCursorPosition) {
		return '';
	}

	// previousLineCount counts the trailing newline, so the last visible line is
	// previousLineCount - 1; walk down from the cursor's row to reach it.
	const linesDown = previousLineCount - 1 - previousCursorPosition.y;

	return (
		(linesDown > 0 ? ansiEscapes.cursorDown(linesDown) : '') +
		ansiEscapes.cursorTo(0)
	);
};
