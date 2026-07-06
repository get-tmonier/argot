// Break: workerpool pool (bare callee) where outline runs background work via Bull queues.
export async function renderDocumentsConcurrently(docs: string[]) {
  const pool = workerpool.pool("./renderWorker.js", { maxWorkers: 4 });
  try {
    return await Promise.all(
      docs.map((doc) => pool.exec("renderMarkdown", [doc]))
    );
  } finally {
    await pool.terminate();
  }
}
