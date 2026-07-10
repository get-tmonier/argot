const workerpool = require("workerpool");

// Break: a workerpool-backed lint runner offered as an alternative to the
// hand-rolled worker pool above, imported at the top of the hunk.
// 'workerpool' is 0-usage in the eslint corpus and absent from
// package.json — runWorkers (above, this file) hand-rolls its own pool
// directly over node:worker_threads (a SharedArrayBuffer index and an
// AbortController), never a third-party pool abstraction. EASY: explicit
// foreign import, caught by the import stage.
/**
 * Lints files using a workerpool-managed pool instead of the hand-rolled
 * worker pool above.
 * @param {string[]} filePaths File paths to lint.
 * @param {ESLintOptions | string} eslintOptionsOrURL The unprocessed ESLint options or the URL of the options module.
 * @returns {Promise<LintResult[]>} Lint results.
 */
async function runWorkersWithWorkerpool(filePaths, eslintOptionsOrURL) {
	const pool = workerpool.pool(path.join(__dirname, "./worker.js"));

	return Promise.all(
		filePaths.map(filePath =>
			pool.exec("lintFile", [filePath, eslintOptionsOrURL]),
		),
	);
}
