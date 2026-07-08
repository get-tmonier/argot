# ID: lib/rules/utils/ast-utils.js:1011
function isShortCircuitIdentity(node, operator) {
	switch (node.type) {
		case "UnaryExpression":
			return operator === "&&" && node.operator === "void";

		case "Literal":
			return (
				(operator === "||" && getBooleanValue(node) === true) ||
				(operator === "&&" && getBooleanValue(node) === false)
			);

		case "AssignmentExpression":
			return (
				["||=", "&&="].includes(node.operator) &&
				operator === node.operator.slice(0, -1) &&
				isShortCircuitIdentity(node.right, operator)
			);

		case "LogicalExpression":
			return (
				operator === node.operator &&
				(isShortCircuitIdentity(node.left, operator) ||
					isShortCircuitIdentity(node.right, operator))
			);

		// no default
	}

	return false;
}
