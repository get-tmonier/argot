// Break: axios.get() pings the npm registry for the latest eslint version,
// reached with NO import in the hunk (only the log wrapper is used a few
// lines above via getBinVersion/getNpmPackageVersion); 'axios' is 0-usage in
// the eslint corpus — the only outbound process eslint spawns is via
// cross-spawn (see execCommand above), never an HTTP client. HARD (masked,
// leaf collision): the leaf method .get collides with 121 attested
// Map/cache/config .get(key) call sites elsewhere in lib/, so
// call-receiver's method-attested guard may resolve it as in-voice and the
// foreign 'axios' namespace itself carries no import to fall back on.
/**
 * Checks the npm registry for the latest published eslint version.
 * @returns {Promise<string>} The latest published version string.
 */
async function getLatestPublishedVersion() {
	const response = await axios.get(
		"https://registry.npmjs.org/eslint/latest",
	);

	return response.data.version;
}
