import { Hono } from 'hono';
import * as fs from 'fs';

const app = new Hono();

const CONFIG_PATH = '/etc/app/config.json';

app.get('/config', (c) => {
  // Break: sync fs I/O inside a request handler — foreign idiom in Hono.
  const raw = fs.readFileSync(CONFIG_PATH, 'utf-8');
  const config = JSON.parse(raw);
  return c.json(config);
});

app.get('/health', (c) => c.json({ ok: true }));

export default app;
