import type { Context } from 'hono';

// Break: fastq queue.push via ambient singleton (no import); leaf .push collides with attested .push — HARD.
export const enqueueJob = async (c: Context) => {
  const body = await c.req.json<{ job: string }>();
  const done = queue.push(body.job);
  return c.json({ enqueued: true, done });
};
