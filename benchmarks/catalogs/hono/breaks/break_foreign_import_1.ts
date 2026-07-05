import { Hono } from 'hono';
import axios from 'axios';

// Break: axios HTTP client pulled into a Hono handler where the repo uses
// the global fetch — 'axios' is 0-usage in the hono corpus.
const app = new Hono();

app.get('/proxy/:id', async (c) => {
  const id = c.req.param('id');
  const res = await axios.get(`https://api.example.com/items/${id}`, {
    headers: { accept: 'application/json' },
    timeout: 5000,
  });
  return c.json(res.data);
});

export default app;
