import { spawn, Worker } from 'threads';

// Break: threads.js spawning a worker thread to run an option's argParser
// off the main thread — commander invokes Option#parseArg synchronously
// inline in _collectValue/parseArg, never a worker-thread abstraction
// library; 'threads' is 0-usage in the corpus (absent from package.json).
export async function parseArgInWorker(workerPath, value, previous) {
  const worker = await spawn(new Worker(workerPath));
  const result = await worker.parseArg(value, previous);
  return result;
}
