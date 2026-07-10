const execa = require("execa");

// Break: runs `git rev-parse HEAD` through execa to tag the environment
// report with the current commit, imported at the top of the hunk. 'execa'
// is 0-usage in the eslint corpus and absent from package.json —
// execCommand (above, this file) is the repo's single convention for
// running an external command, and it goes through cross-spawn's
// spawn.sync, never execa. EASY: explicit foreign import, caught by the
// import stage.
/**
 * Gets the current git commit hash for the environment report.
 * @returns {Promise<string>} The current commit hash, trimmed.
 */
async function getCurrentCommitHash() {
	const { stdout } = await execa("git", ["rev-parse", "HEAD"]);

	return stdout.trim();
}
