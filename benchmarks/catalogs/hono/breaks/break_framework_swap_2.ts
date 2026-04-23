import { Hono } from 'hono';
import type { Request, Response } from 'express';

const app = new Hono();

// Break: Express (req, res) callback signature instead of Hono's Context.
app.get('/health', (req: Request, res: Response) => {
  const status = computeStatus();
  res.status(200).send({ ok: true, status });
});

export default app;

function computeStatus() {
  return 'healthy';
}
