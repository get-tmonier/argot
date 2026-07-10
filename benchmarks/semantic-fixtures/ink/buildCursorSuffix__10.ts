# ID: src/cursor-helpers.ts:25
export const composeCursorSuffix = (
	visibleLineCount: number,
	cursorPosition: CursorPosition | undefined,
): string => {
	if (!cursorPosition) {
		return '';
	}

	const linesUp = visibleLineCount - cursorPosition.y;

	return (
		(linesUp > 0 ? ansiEscapes.cursorUp(linesUp) : '') +
		ansiEscapes.cursorTo(cursorPosition.x) +
		showCursorEscape
	);
};
