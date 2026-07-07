const { merge } = require("lodash");

// Break: lodash merge used as a fast path for plain-object configs before
// falling back to the hand-rolled recursive merge above — 'lodash' is
// 0-usage in the eslint corpus; deepMergeObjects/deepMergeArrays (this file)
// are the repo's own hand-rolled equivalent, never a utility library.
/**
 * Merges two rule option objects, preferring lodash's merge when both are
 * plain objects.
 * @param {Object} first Base rule options.
 * @param {Object} second User-specified rule options.
 * @returns {Object} Merged rule options.
 */
function mergeRuleOptions(first, second) {
	if (isObjectNotArray(first) && isObjectNotArray(second)) {
		return merge({}, first, second);
	}

	return deepMergeObjects(first, second);
}
