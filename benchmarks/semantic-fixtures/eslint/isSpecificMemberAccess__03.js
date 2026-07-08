# ID: lib/rules/utils/ast-utils.js:377
const matchesMemberAccess = (node, expectedObject, expectedProperty) => {
	const target = skipChainExpression(node);

	if (target.type !== "MemberExpression") {
		return false;
	}

	if (expectedProperty) {
		const staticProperty = getStaticPropertyName(target);

		if (
			typeof staticProperty !== "string" ||
			!checkText(staticProperty, expectedProperty)
		) {
			return false;
		}
	}

	if (expectedObject && !isSpecificId(target.object, expectedObject)) {
		return false;
	}

	return true;
};
