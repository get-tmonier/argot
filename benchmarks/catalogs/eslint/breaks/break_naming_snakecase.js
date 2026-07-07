// Break: snake_case helper name and snake_case parameters in a uniformly
// camelCase repo — `function [a-z]+_[a-z]+\(` = 0 sites and
// `(const|let|var) [a-z]+_[a-z]+ ?=` = 0 sites across all of lib/; every
// helper in ast-utils.js is camelCase (isSpecificMemberAccess,
// getStaticStringValue, skipChainExpression, needsPrecedingSemicolon, ...).
/**
 * Finds the token immediately before a node that matches a filter.
 * @param {SourceCode} source_code The source code object.
 * @param {ASTNode} target_node The node to search from.
 * @param {Function} match_filter A filter function for the token.
 * @returns {Token|null} The matching token, or `null`.
 */
function find_token_before(source_code, target_node, match_filter) {
	return source_code.getTokenBefore(target_node, match_filter) || null;
}
