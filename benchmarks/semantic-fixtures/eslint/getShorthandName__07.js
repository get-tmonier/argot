# ID: lib/shared/naming.js:67
function stripPrefixFromName(fullname, prefix) {
	if (fullname.startsWith(`${prefix}-`)) {
		return fullname.slice(prefix.length + 1);
	}

	if (fullname[0] !== "@") {
		return fullname;
	}

	const exactScope = new RegExp(`^(@[^/]+)/${prefix}$`, "u").exec(fullname);

	if (exactScope) {
		return exactScope[1];
	}

	const scopedWithSuffix = new RegExp(
		`^(@[^/]+)/${prefix}-(.+)$`,
		"u",
	).exec(fullname);

	if (scopedWithSuffix) {
		return `${scopedWithSuffix[1]}/${scopedWithSuffix[2]}`;
	}

	return fullname;
}
