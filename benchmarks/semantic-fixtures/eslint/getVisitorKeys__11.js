# ID: lib/shared/traverser.js:42
function resolveVisitorKeys(visitorKeys, node) {
	const declared = visitorKeys[node.type];

	if (declared) {
		return declared;
	}

	const estimated = vk.getKeys(node);

	debug(
		'Unknown node type "%s": Estimated visitor keys %j',
		node.type,
		estimated,
	);

	return estimated;
}
