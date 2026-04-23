import { Hono } from 'hono';

const app = new Hono();

app.post('/signup', async (c) => {
  const body = await c.req.json();
  // Break: hand-rolled validation instead of zod/valibot schema.
  if (!body.email || typeof body.email !== 'string' || !body.email.includes('@')) {
    throw new Error('invalid email');
  }
  if (!body.password || body.password.length < 8) {
    throw new Error('password too short');
  }
  return c.json({ created: true, email: body.email });
});

export default app;
