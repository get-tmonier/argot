import type { Context } from 'hono';

// Break: tinypool pool.run via ambient singleton (no import); leaf .run collides with attested .run — HARD.
export const offload = async (c: Context) => {
  const body = await c.req.json<{ task: string }>();
  const result = await pool.run(body.task);
  return c.json({ result });
};
