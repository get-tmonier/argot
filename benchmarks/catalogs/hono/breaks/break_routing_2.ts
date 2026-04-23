import { Hono } from 'hono';
import { Router } from 'express';

const app = new Hono();

// Break: Router() + router.route(...).get().post() chain composition.
const router = Router();
router
  .route('/users/:id')
  .get((req, res) => res.json({ id: req.params.id }))
  .post((req, res) => res.status(201).json({ id: req.params.id }))
  .delete((req, res) => res.status(204).send());

app.use('/api', router);

export default app;
