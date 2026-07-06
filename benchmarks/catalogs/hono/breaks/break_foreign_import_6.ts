import { Hono } from 'hono';
import groupBy from 'lodash/groupBy';

// Break: lodash/groupBy submodule import for aggregation (repo uses reduce/Map) — 0-usage.
const app = new Hono();

app.post('/summary', async (c) => {
  const body = await c.req.json<{ items: Array<{ kind: string }> }>();
  const grouped = groupBy(body.items, (item) => item.kind);
  return c.json({ groups: Object.keys(grouped) });
});

export default app;
