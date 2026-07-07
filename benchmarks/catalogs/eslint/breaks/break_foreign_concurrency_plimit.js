const pLimit = require("p-limit");

// Break: bounds the per-file read/lint fan-out with p-limit — 'p-limit' is
// 0-usage in the eslint corpus; lintFilesWithoutMultithreading (above)
// already bounds concurrency with the repo's own @humanwhocodes/retry
// Retrier (`concurrency: 100`), never a third-party limiter.
/**
 * Lints files with a bounded concurrency using p-limit.
 * @param {ESLint} eslint ESLint instance.
 * @param {string[]} filePaths File paths to lint.
 * @returns {Promise<LintResult[]>} Lint results.
 */
async function lintFilesWithConcurrencyLimit(eslint, filePaths) {
	const limit = pLimit(20);

	return Promise.all(
		filePaths.map(filePath =>
			limit(() => lintFilesWithoutMultithreading(eslint, [filePath])),
		),
	);
}
