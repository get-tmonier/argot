# ID: lib/rules/utils/fix-tracker.js:94
function buildRangeReplacement(tracker, range, text) {
	let actualRange = range;

	if (tracker.retainedRange) {
		actualRange = [
			Math.min(tracker.retainedRange[0], range[0]),
			Math.max(tracker.retainedRange[1], range[1]),
		];
	}

	const prefix = tracker.sourceCode.text.slice(actualRange[0], range[0]);
	const suffix = tracker.sourceCode.text.slice(range[1], actualRange[1]);

	return tracker.fixer.replaceTextRange(actualRange, prefix + text + suffix);
}
