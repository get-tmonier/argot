import { Hono } from 'hono';
import slugify from 'slugify';

// Break: slugify slug generator in a Hono handler (repo has no slug dependency) — 0-usage.
const app = new Hono();

app.post('/posts', async (c) => {
  const body = await c.req.json<{ title: string }>();
  const slug = slugify(body.title, { lower: true, strict: true });
  return c.json({ slug });
});

export default app;
