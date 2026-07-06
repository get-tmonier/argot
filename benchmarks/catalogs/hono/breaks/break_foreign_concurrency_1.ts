import { Hono } from 'hono';
import PQueue from 'p-queue';

// Break: p-queue concurrency limiter driving request work where the repo uses
// plain async/await — 'p-queue'/PQueue are 0-usage in the hono corpus.
const app = new Hono();

const queue = new PQueue({ concurrency: 4 });

app.post('/batch', async (c) => {
  const body = await c.req.json<{ items: string[] }>();
  const results = await queue.addAll(
    body.items.map((item) => async () => {
      return { item, processed: true };
    }),
  );
  return c.json({ count: results.length, results });
});

export default app;
