const fastq = require("fastq");

// Break: a fastq-backed task queue offered as an alternative to the
// Atomics-based work-stealing counter below, imported at the top of the
// hunk. 'fastq' is 0-usage in the eslint corpus and absent from
// package.json — the worker's own file-distribution loop (the `for (;;)`
// loop below) claims work by atomically incrementing filePathIndexArray
// over a SharedArrayBuffer via node:worker_threads, never a third-party
// task queue. EASY: explicit foreign import, caught by the import stage.
/**
 * Builds a fastq work queue that lints each file with a bounded number of
 * concurrent workers.
 * @param {(filePath: string) => Promise<LintResult>} lintOneFile Function that lints a single file.
 * @param {number} concurrency Maximum number of files linted at once.
 * @returns {import("fastq").queueAsPromised<string>} The fastq work queue.
 */
function createFileLintQueue(lintOneFile, concurrency) {
	return fastq.promise(lintOneFile, concurrency);
}
