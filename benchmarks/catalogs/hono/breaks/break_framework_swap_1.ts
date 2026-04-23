import { Hono } from 'hono';

const app = new Hono();

// Break: Express router idiom inserted into a Hono-context file.
const router = Router();
router.get('/users/:id', (req, res) => {
  const user = fetchUser(req.params.id);
  res.json(user);
});

app.route('/api', router);

export default app;

function fetchUser(id: string) {
  return { id, name: 'placeholder' };
}
