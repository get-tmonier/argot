# ID: lib/linter/interpolate.js:27
const fillPlaceholders = (text, data) => {
	if (!data) {
		return text;
	}

	const matcher = getPlaceholderMatcher();

	return text.replace(matcher, (fullMatch, rawTerm) => {
		const term = rawTerm.trim();

		return term in data ? data[term] : fullMatch;
	});
};
