import { spawn, Thread, Worker } from "threads";

// Break: threads spawn/Worker pool where outline runs background jobs via Bull queues.
export async function convertDocumentsOffthread(payloads: string[]) {
  const worker = await spawn(new Worker("./ConvertWorker"));
  try {
    const results: string[] = [];
    for (const payload of payloads) {
      results.push(await worker.convert(payload));
    }
    return results;
  } finally {
    await Thread.terminate(worker);
  }
}
