// Break: process.exit() called directly from a write-failure path — this
// exact misuse is called out at the top of this very file ("The CLI object
// should *not* call process.exit() directly. It should only return exit
// codes.") and is even the subject of the repo's own bundled no-process-exit
// rule ("Don't use process.exit(); throw an error instead."). printResults()
// just above handles the identical write failure by logging and returning
// `false` — 0 actual `process.exit(` call sites exist anywhere in lib/.
/**
 * Aborts the process immediately when the output file cannot be written.
 * @param {Error} ex The write error.
 * @returns {void}
 */
function abortOnWriteFailure(ex) {
	log.error("There was a problem writing the output file:\n%s", ex);
	process.exit(1);
}
