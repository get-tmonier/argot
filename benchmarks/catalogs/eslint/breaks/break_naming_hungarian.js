// Break: Hungarian-prefixed locals/params (objInput, objOriginal, bIsEqual,
// strKey, arrKeys) in a file whose own containsDifferentProperty (above)
// uses plain camelCase (input, original, inputKeys, originalKeys). No
// simultaneous obj-/str-/arr-/bool-prefixed identifiers co-occur anywhere
// else in lib/shared, and 0 `b[A-Z]... =` boolean-prefixed locals exist in
// lib/ at all.
/**
 * Checks whether two option objects are shallowly equal.
 * @param {Object} objInput The new options object.
 * @param {Object} objOriginal The original options object.
 * @returns {boolean} Whether the two objects are equal.
 */
function checkOptionsEquality(objInput, objOriginal) {
	const arrKeys = Object.keys(objInput);
	let bIsEqual = arrKeys.length === Object.keys(objOriginal).length;

	for (const strKey of arrKeys) {
		bIsEqual = bIsEqual && objInput[strKey] === objOriginal[strKey];
	}

	return bIsEqual;
}
