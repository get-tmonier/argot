# ID: lib/linter/apply-disable-directives.js:40
function clusterByParentDirective(directives) {
	const buckets = new Map();

	directives.forEach(directive => {
		const {
			unprocessedDirective: { parentDirective },
		} = directive;

		const existing = buckets.get(parentDirective);

		if (existing) {
			existing.push(directive);
		} else {
			buckets.set(parentDirective, [directive]);
		}
	});

	return [...buckets.values()];
}
