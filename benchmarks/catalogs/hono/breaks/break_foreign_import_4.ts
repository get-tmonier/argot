import { Hono } from 'hono';
import { load } from 'js-yaml';

// Break: js-yaml load() config parser in a Hono handler (repo parses JSON) — 0-usage.
const app = new Hono();

app.post('/config', async (c) => {
  const raw = await c.req.text();
  const parsed = load(raw) as Record<string, unknown>;
  return c.json({ keys: Object.keys(parsed) });
});

export default app;
