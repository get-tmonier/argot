import PQueue from 'p-queue';

// Break: a p-queue promise queue fanning subcommand hooks out with a
// concurrency cap — commander chains hooks sequentially via
// _chainOrCallSubCommandHook, never a promise-queue dependency; 'p-queue'
// is 0-usage in the corpus (absent from package.json).
const queue = new PQueue({ concurrency: 4 });

export function runSubCommandHooksQueued(hookFns) {
  return Promise.all(hookFns.map((fn) => queue.add(fn)));
}
