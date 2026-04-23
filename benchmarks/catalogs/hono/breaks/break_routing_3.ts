import { Hono } from 'hono';

const app = new Hono();

app.get('/ping', (c) => c.text('pong'));

// Break: Express-style app.all('*', handler) wildcard catch-all.
app.all('*', (req, res) => {
  res.status(404).json({
    error: 'not found',
    path: req.originalUrl,
    method: req.method,
  });
});

export default app;
