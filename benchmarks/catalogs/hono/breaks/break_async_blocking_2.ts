import { Hono } from 'hono';
import * as fs from 'fs';

const app = new Hono();

app.get('/download/:name', (c) => {
  const name = c.req.param('name');
  // Break: synchronous readFileSync inside a streaming endpoint.
  const buffer = fs.readFileSync(`/var/data/${name}.bin`);
  const stats = fs.statSync(`/var/data/${name}.bin`);
  return c.body(buffer, 200, {
    'Content-Length': String(stats.size),
    'Content-Type': 'application/octet-stream',
  });
});

export default app;
