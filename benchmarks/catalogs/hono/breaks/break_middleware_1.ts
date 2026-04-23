import { Hono } from 'hono';

const app = new Hono();

// Break: Express-style (req, res, next) middleware signature in app.use.
app.use((req, res, next) => {
  const started = Date.now();
  res.on('finish', () => {
    console.log(`${req.method} ${req.url} took ${Date.now() - started}ms`);
  });
  next();
});

app.get('/ping', (c) => c.text('pong'));

export default app;
