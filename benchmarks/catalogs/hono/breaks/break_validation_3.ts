import { Hono } from 'hono';

const app = new Hono();

const PHONE_RE = /^\+?[1-9]\d{1,14}$/;
const ZIP_RE = /^\d{5}(-\d{4})?$/;

app.post('/contact', async (c) => {
  const body = await c.req.json();
  // Break: regex-literal validation instead of a validator library.
  if (!PHONE_RE.test(body.phone ?? '')) {
    return c.json({ error: 'bad phone' }, 400);
  }
  if (!ZIP_RE.test(body.zip ?? '')) {
    return c.json({ error: 'bad zip' }, 400);
  }
  return c.json({ saved: true });
});

export default app;
