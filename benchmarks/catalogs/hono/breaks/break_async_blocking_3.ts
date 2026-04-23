import { Hono } from 'hono';
import { execSync } from 'child_process';

const app = new Hono();

app.post('/git/status', (c) => {
  // Break: synchronous child_process.execSync blocks the event loop per request.
  const out = execSync('git status --porcelain', { encoding: 'utf-8' });
  const branch = execSync('git rev-parse --abbrev-ref HEAD', { encoding: 'utf-8' });
  return c.json({ branch: branch.trim(), dirty: out.length > 0 });
});

app.get('/health', (c) => c.json({ ok: true }));

export default app;
