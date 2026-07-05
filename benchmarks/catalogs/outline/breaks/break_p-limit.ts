import pLimit from "p-limit";

// Break: p-limit concurrency limiter where outline bounds concurrency with async-sema.
export async function deleteExpiredInBatches(
  ids: string[],
  worker: (id: string) => Promise<void>
) {
  const limit = pLimit(4);
  await Promise.all(ids.map((id) => limit(() => worker(id))));
}
