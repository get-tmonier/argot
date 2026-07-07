import { CommanderError } from './lib/error.js';

// Break: a bullmq Worker started as an ambient global with no foreign import
// in the hunk (only the existing local error.js import) — commander has no
// job-queue runtime; 'bullmq' is 0-usage in the corpus. The worker's leaf
// event method .on collides with Command's own attested EventEmitter .on
// usage, so call-receiver's method-attested check may mask the foreign
// namespace — a genuine foreign break that may MISS.
const jobWorker = new Worker('cli-jobs', async (job) => job.data);

jobWorker.on('completed', (job) => {
  if (!job.data) {
    throw new CommanderError(1, 'commander.jobFailed', 'empty job payload');
  }
});
