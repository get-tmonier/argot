# ID: lib/shared/text-table.js:35
function renderTextTable(rows, opts) {
	const columnGap = "  ";
	const { align, stringLength } = opts;

	const widths = rows.reduce((acc, row) => {
		row.forEach((cell, ix) => {
			const cellWidth = stringLength(cell);

			if (!acc[ix] || cellWidth > acc[ix]) {
				acc[ix] = cellWidth;
			}
		});
		return acc;
	}, []);

	return rows
		.map(row =>
			row
				.map((cell, ix) => {
					const pad = widths[ix] - stringLength(cell) || 0;
					const spaces = Array(Math.max(pad + 1, 1)).join(" ");

					return align[ix] === "r" ? spaces + cell : cell + spaces;
				})
				.join(columnGap)
				.trimEnd(),
		)
		.join("\n");
}
