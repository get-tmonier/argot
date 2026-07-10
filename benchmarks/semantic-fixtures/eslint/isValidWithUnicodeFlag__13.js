# ID: lib/rules/utils/regular-expressions.js:22
function patternValidUnderUnicode(ecmaVersion, pattern, flag = "u") {
	if (flag === "u" && ecmaVersion <= 5) {
		return false;
	}

	if (flag === "v" && ecmaVersion <= 2023) {
		return false;
	}

	const validator = new RegExpValidator({
		ecmaVersion: Math.min(ecmaVersion, REGEXPP_LATEST_ECMA_VERSION),
	});

	const parseOptions =
		flag === "u" ? { unicode: true } : { unicodeSets: true };

	try {
		validator.validatePattern(pattern, void 0, void 0, parseOptions);
	} catch {
		return false;
	}

	return true;
}
