const Piscina = require("piscina");

// Break: a piscina worker-pool wrapper offered as an alternative lint runner
// — 'piscina' is 0-usage in the eslint corpus; the repo hand-rolls its own
// worker pool directly over node:worker_threads (see runWorkers/Worker
// above), never a third-party pool abstraction.
/**
 * Lints files using a piscina worker pool instead of the hand-rolled
 * worker pool above.
 * @param {string[]} filePaths File paths to lint.
 * @param {ESLintOptions | string} eslintOptionsOrURL The unprocessed ESLint options or the URL of the options module.
 * @returns {Promise<LintResult[]>} Lint results.
 */
async function runWorkersWithPiscina(filePaths, eslintOptionsOrURL) {
	const pool = new Piscina({
		filename: path.join(__dirname, "./worker.js"),
	});

	return Promise.all(
		filePaths.map(filePath => pool.run({ filePath, eslintOptionsOrURL })),
	);
}
