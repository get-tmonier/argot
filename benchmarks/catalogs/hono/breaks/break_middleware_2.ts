import { Hono } from 'hono';

const app = new Hono();

app.get('/boom', () => {
  throw new Error('nope');
});

// Break: Express 4-arg (err, req, res, next) error-handler signature.
app.use((err, req, res, next) => {
  console.error('error', err);
  res.status(500).json({ error: err.message });
  next(err);
});

export default app;
