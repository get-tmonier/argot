import { Hono } from 'hono';
import Redis from 'ioredis';

// Break: ioredis client and its .setex/.get API used for caching where the
// repo has no Redis dependency — 'ioredis'/new Redis are 0-usage in the corpus.
const app = new Hono();

const redis = new Redis(process.env.REDIS_URL ?? 'redis://localhost:6379');

app.get('/cache/:key', async (c) => {
  const key = c.req.param('key');
  const cached = await redis.get(key);
  if (cached) {
    return c.json({ hit: true, value: JSON.parse(cached) });
  }
  const value = { at: Date.now() };
  await redis.setex(key, 60, JSON.stringify(value));
  return c.json({ hit: false, value });
});

export default app;
