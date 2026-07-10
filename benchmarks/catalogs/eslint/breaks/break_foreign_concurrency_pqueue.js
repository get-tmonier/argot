const PQueue = require("p-queue");

// Break: bounds the concurrent glob-pattern resolution with a p-queue
// queue, imported at the top of the hunk. 'p-queue' is 0-usage in the
// eslint corpus and absent from package.json — findFiles (above, this
// file) resolves every stat()/globMultiSearch call through a plain
// Promise.all with no concurrency limiter at all, and the repo's one
// bounded-concurrency convention (lintFilesWithoutMultithreading in
// lib/eslint/eslint.js) uses its own @humanwhocodes/retry Retrier, never a
// third-party queue. EASY: explicit foreign import, caught by the import
// stage.
/**
 * Resolves a list of glob patterns with a bounded number of concurrent
 * lookups.
 * @param {string[]} patterns Glob patterns to resolve.
 * @param {(pattern: string) => Promise<string[]>} resolveOne Function that resolves a single pattern.
 * @returns {Promise<string[]>} The resolved file paths.
 */
async function resolvePatternsWithQueue(patterns, resolveOne) {
	const queue = new PQueue({ concurrency: 8 });

	const results = await Promise.all(
		patterns.map(pattern => queue.add(() => resolveOne(pattern))),
	);

	return results.flat();
}
