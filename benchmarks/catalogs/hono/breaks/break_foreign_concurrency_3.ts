import { Hono } from 'hono';
import pLimit from 'p-limit';

// Break: p-limit concurrency limiter in a Hono handler — 0-usage.
const app = new Hono();

const limit = pLimit(5);

app.post('/fanout', async (c) => {
  const body = await c.req.json<{ ids: string[] }>();
  const results = await Promise.all(
    body.ids.map((id) => limit(async () => ({ id, ok: true })))
  );
  return c.json({ results });
});

export default app;
