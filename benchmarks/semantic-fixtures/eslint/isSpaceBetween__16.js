# ID: lib/languages/js/source-code/source-code.js:504
function hasWhitespaceBetween(sourceCode, first, second) {
	if (nodesOrTokensOverlap(first, second)) {
		return false;
	}

	const [leading, trailing] =
		first.range[1] <= second.range[0] ? [first, second] : [second, first];

	let currentToken = sourceCode.getLastToken(leading) || leading;
	const finalToken = sourceCode.getFirstToken(trailing) || trailing;

	while (currentToken !== finalToken) {
		const nextToken = sourceCode.getTokenAfter(currentToken, {
			includeComments: true,
		});

		if (currentToken.range[1] !== nextToken.range[0]) {
			return true;
		}

		currentToken = nextToken;
	}

	return false;
}
