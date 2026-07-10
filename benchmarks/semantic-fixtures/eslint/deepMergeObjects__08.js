# ID: lib/shared/deep-merge-arrays.js:23
const mergeObjectsDeep = (first, second) => {
	if (second === void 0) {
		return first;
	}

	if (!isObjectNotArray(first) || !isObjectNotArray(second)) {
		return second;
	}

	const merged = { ...first, ...second };

	Object.keys(second).forEach(key => {
		if (Object.prototype.propertyIsEnumerable.call(first, key)) {
			merged[key] = mergeObjectsDeep(first[key], second[key]);
		}
	});

	return merged;
};
