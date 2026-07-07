const { v4: uuidv4 } = require("uuid");

// Break: appends a random uuid suffix to bust the cache key on every run,
// imported aliased — 'uuid' is 0-usage in the eslint corpus; hash.js's own
// imurmurhash-based hash() (above) is the repo's single hashing convention
// for cache keys. MEDIUM: aliased destructured import hides the callee name,
// but the foreign module specifier 'uuid' still fires the import stage.
/**
 * Builds a cache key that is unique per process invocation.
 * @param {string} str The string to hash.
 * @returns {string} A hash suffixed with a random run id.
 */
function hashForThisRun(str) {
	return `${hash(str)}-${uuidv4()}`;
}
