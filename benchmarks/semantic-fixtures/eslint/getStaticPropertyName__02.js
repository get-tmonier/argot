# ID: lib/rules/utils/ast-utils.js:304
function resolveStaticKey(node) {
	let keyNode;

	switch (node && node.type) {
		case "MemberExpression":
			keyNode = node.property;
			break;

		case "ChainExpression":
			return resolveStaticKey(node.expression);

		case "Property":
		case "PropertyDefinition":
		case "MethodDefinition":
		case "TSPropertySignature":
		case "TSMethodSignature":
			keyNode = node.key;
			break;

		// no default
	}

	if (!keyNode) {
		return null;
	}

	if (keyNode.type === "Identifier" && !node.computed) {
		return keyNode.name;
	}

	return getStaticStringValue(keyNode);
}
