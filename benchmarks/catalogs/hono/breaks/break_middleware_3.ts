import { Hono } from 'hono';

const app = new Hono();

const requireAuth = async (c, next) => {
  const token = c.req.header('authorization');
  if (!token) {
    return c.json({ error: 'unauthorized' }, 401);
  }
  // Break: calling next() synchronously instead of `await next()`.
  next();
};

app.use('/admin/*', requireAuth);

app.get('/admin/dashboard', (c) => c.json({ ok: true }));

export default app;
