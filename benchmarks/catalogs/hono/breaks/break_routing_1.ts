import { Hono } from 'hono';
import { Router } from 'express';

const app = new Hono();

// Break: express.Router() composed under app.use('/api', router).
const router = Router();
router.get('/items', (req, res) => res.json([]));
router.post('/items', (req, res) => res.status(201).json({ created: true }));
router.delete('/items/:id', (req, res) => res.status(204).send());

app.use('/api', router);

app.get('/health', (c) => c.json({ ok: true }));

export default app;
