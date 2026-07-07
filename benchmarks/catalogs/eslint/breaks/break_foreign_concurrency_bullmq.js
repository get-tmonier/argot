// Break: enqueues per-file stats onto a bullmq queue for async dashboard
// ingestion, reached with NO import in the hunk; 'bullmq' is 0-usage in the
// eslint corpus — calculateStatsPerFile (above) returns its stat object
// in-process and eslint has no background-job runtime at all. HARD (masked,
// leaf collision): the leaf method .add collides with 128 attested
// Set/Map/array .add(...) call sites elsewhere in lib/, so call-receiver's
// method-attested guard may resolve it as in-voice and the foreign 'bullmq'
// namespace itself carries no import to fall back on.
/**
 * Queues per-file stats for asynchronous dashboard ingestion.
 * @param {LintMessage[]} messages Collection of messages.
 * @returns {Promise<void>} Resolves once the job is queued.
 */
async function queueStatsForDashboard(messages) {
	const stat = calculateStatsPerFile(messages);

	await statsQueue.add("file-stats", stat);
}
