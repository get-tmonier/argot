# ID: src/utils.ts:8
export const resolveTerminalDimensions = (
	stdout: NodeJS.WriteStream,
): {columns: number; rows: number} => {
	// `columns`/`rows` can be 0 or undefined when stdout is not a TTY.
	const {columns, rows} = stdout;

	if (columns && rows) {
		return {columns, rows};
	}

	const fallback = terminalSize();
	return {
		columns: columns || fallback.columns || 80,
		rows: rows || fallback.rows || 24,
	};
};
