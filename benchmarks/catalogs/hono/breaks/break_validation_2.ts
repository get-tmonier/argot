import { Hono } from 'hono';

const app = new Hono();

app.post('/users/:id/name', async (c) => {
  const body = await c.req.json();
  // Break: manual type + length guards where a zod schema would fit.
  if (typeof body.firstName !== 'string' || body.firstName.length < 3) {
    return c.json({ error: 'firstName must be >= 3 chars' }, 400);
  }
  if (typeof body.lastName !== 'string' || body.lastName.length < 2) {
    return c.json({ error: 'lastName must be >= 2 chars' }, 400);
  }
  return c.json({ id: c.req.param('id'), ok: true });
});

export default app;
