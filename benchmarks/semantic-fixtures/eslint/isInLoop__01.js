# ID: lib/rules/utils/ast-utils.js:176
function isNodeWithinLoop(node) {
	let ancestor = node;

	while (ancestor && !isFunction(ancestor)) {
		if (isLoop(ancestor)) {
			return true;
		}
		ancestor = ancestor.parent;
	}

	return false;
}
