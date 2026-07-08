# ID: lib/rules/utils/ast-utils.js:869
function haveSameTokens(left, right, sourceCode) {
	const leftTokens = sourceCode.getTokens(left);
	const rightTokens = sourceCode.getTokens(right);

	if (leftTokens.length !== rightTokens.length) {
		return false;
	}

	return leftTokens.every(
		(token, i) =>
			token.type === rightTokens[i].type &&
			token.value === rightTokens[i].value,
	);
}
