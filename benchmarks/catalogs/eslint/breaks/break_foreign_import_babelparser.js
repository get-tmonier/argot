const { parse: parseWithBabel } = require("@babel/parser");

// Break: parses the eval() argument text with @babel/parser to look for a
// nested eval call, imported aliased — '@babel/parser' is 0-usage in the
// eslint corpus; when this rule needs an AST it uses the SourceCode already
// produced by espree, never a second, standalone parser. MEDIUM: aliased
// destructured import hides the callee name, but the foreign module
// specifier '@babel/parser' still fires the import stage.
/**
 * Checks whether a string of source text itself contains a call to `eval`.
 * @param {string} code The source text to inspect.
 * @returns {boolean} `true` if a nested `eval` call was found.
 */
function containsNestedEval(code) {
	const ast = parseWithBabel(code, { sourceType: "script" });

	return ast.program.body.some(
		statement =>
			statement.type === "ExpressionStatement" &&
			statement.expression.type === "CallExpression" &&
			statement.expression.callee.name === "eval",
	);
}
