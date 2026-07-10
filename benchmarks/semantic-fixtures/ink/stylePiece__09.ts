# ID: src/render-border.ts:7
const decorateSegment = (
	segment: string,
	fg?: string,
	bg?: string,
	dim?: boolean,
): string => {
	let decorated = colorize(segment, fg, 'foreground');
	decorated = colorize(decorated, bg, 'background');

	if (dim) {
		decorated = chalk.dim(decorated);
	}

	return decorated;
};
