import Piscina from "piscina";
import { resolve } from "node:path";

const pool = new Piscina({
  filename: resolve(__dirname, "importWorker.js"),
  maxThreads: 4,
});

// Break: piscina worker pool where outline offloads background work to Bull queues.
export async function importInParallel(chunks: string[][]) {
  return Promise.all(chunks.map((chunk) => pool.run(chunk)));
}
