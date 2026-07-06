// Break: bypasses this file's own `log` wrapper (used a few lines below in
// getBinVersion/getNpmPackageVersion) and calls console.error directly —
// every other error path in runtime-info.js goes through log.error, and
// console.error/console.warn/console.log never appear directly in
// production lib/ code outside of shared/logging.js itself (the sole
// exception, linter/timing.js, has an explicit
// `eslint-disable-line no-console` marking it a deliberate debug escape
// hatch).
/**
 * Logs a command failure without going through the repo's log wrapper.
 * @param {string} bin The binary that failed.
 * @param {Error} error The error that was thrown.
 * @returns {void}
 */
function logCommandFailure(bin, error) {
	console.error(`Command failed for ${bin}: ${error.message}`);
}
