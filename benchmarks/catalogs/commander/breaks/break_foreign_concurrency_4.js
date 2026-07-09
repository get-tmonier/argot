import workerpool from 'workerpool';

// Break: a workerpool thread pool fanning lifecycle hook callbacks out
// across worker threads — commander runs hooks sequentially through
// _chainOrCall/_chainOrCallHooks, never a worker-thread pool library;
// 'workerpool' is 0-usage in the corpus (absent from package.json).
const pool = workerpool.pool();

export function runHooksInWorkerPool(hookFns) {
  return Promise.all(hookFns.map((fn) => pool.exec(fn, [])));
}
