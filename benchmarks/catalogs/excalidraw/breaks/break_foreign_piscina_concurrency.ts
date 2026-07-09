import Piscina from "piscina";

// Break: piscina spins up a foreign Node.js worker_threads pool to run glyph
// subsetting where excalidraw's own WorkerPool (workers.ts) already wraps
// browser Web Workers for this; piscina targets Node worker_threads, a
// runtime this browser code doesn't have, and is 0-usage at the pinned SHA
// and absent from package.json.
const subsetPool = new Piscina({
  filename: new URL("./subset-worker.chunk.js", import.meta.url).href,
});

export const subsetWithPiscina = (arrayBuffer: ArrayBuffer) => {
  return subsetPool.run(arrayBuffer);
};
