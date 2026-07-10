# ID: src/reconciler.ts:48
const computePropDiff = (
	before: AnyObject,
	after: AnyObject,
): AnyObject | undefined => {
	if (before === after) {
		return;
	}

	if (!before) {
		return after;
	}

	const changed: AnyObject = {};
	let dirty = false;

	for (const key of Object.keys(before)) {
		const removed = after ? !Object.hasOwn(after, key) : true;

		if (removed) {
			changed[key] = undefined;
			dirty = true;
		}
	}

	if (after) {
		for (const key of Object.keys(after)) {
			if (after[key] !== before[key]) {
				changed[key] = after[key];
				dirty = true;
			}
		}
	}

	return dirty ? changed : undefined;
};
