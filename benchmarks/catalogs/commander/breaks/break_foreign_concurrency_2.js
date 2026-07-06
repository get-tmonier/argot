import Piscina from 'piscina';

// Break: a Piscina worker-thread pool offloading a heavy action handler —
// commander executes subcommands as child processes via node:child_process,
// never a worker-thread pool library; 'piscina' is 0-usage in the corpus.
const pool = new Piscina({
  filename: new URL('./workers/compile.js', import.meta.url).pathname,
});

export function compileInWorker(sourcePath) {
  return pool.run({ sourcePath });
}
