import express, { Router, Request, Response } from 'express';

// Break: standalone Express router module in a Hono project.
// Import at line 1 is the dead giveaway.
const router = Router();

router.get('/ping', (_req: Request, res: Response) => {
  res.json({ ok: true });
});

router.post('/data', (req: Request, res: Response) => {
  res.status(201).json({ received: req.body });
});

export default router;
