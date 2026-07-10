# ID: lib/shared/severity.js:14
function severityToLabel(severity) {
	if ([0, "0", "off"].includes(severity)) {
		return "off";
	}

	if ([1, "1", "warn"].includes(severity)) {
		return "warn";
	}

	if ([2, "2", "error"].includes(severity)) {
		return "error";
	}

	throw new Error(`Invalid severity value: ${severity}`);
}
