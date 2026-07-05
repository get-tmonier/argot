import { Hono } from 'hono';
import { customAlphabet } from 'nanoid';

// Break: nanoid customAlphabet id generator in a Hono handler (repo uses node:crypto) — 0-usage.
const app = new Hono();

const nano = customAlphabet('0123456789abcdef', 16);

app.post('/tokens', (c) => {
  const token = nano();
  return c.json({ token });
});

export default app;
