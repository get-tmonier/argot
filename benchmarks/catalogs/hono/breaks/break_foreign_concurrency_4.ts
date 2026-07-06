import { Hono } from 'hono';
import Piscina from 'piscina';

// Break: piscina worker-thread pool in a Hono handler — 0-usage.
const app = new Hono();

const pool = new Piscina({ filename: '/app/worker.js' });

app.post('/hash', async (c) => {
  const body = await c.req.json<{ payload: string }>();
  const digest = await pool.run(body.payload);
  return c.json({ digest });
});

export default app;
