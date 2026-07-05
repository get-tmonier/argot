import type { Context } from 'hono';

// Break: new Bottleneck() foreign constructor (no import) in a Hono handler — 0-usage.
const limiter = new Bottleneck({ maxConcurrent: 4, minTime: 200 });

export const throttled = async (c: Context) => {
  const url = c.req.query('url');
  const result = await limiter.schedule(async () => ({ url, ok: true }));
  return c.json({ result });
};
