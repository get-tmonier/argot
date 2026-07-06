import { Hono } from 'hono';
import mysql from 'mysql2/promise';

// Break: mysql2 createPool + SQL query in a Hono handler — 0-usage.
const app = new Hono();

const pool = mysql.createPool({ host: '127.0.0.1', user: 'app', database: 'app' });

app.get('/rows/:id', async (c) => {
  const id = c.req.param('id');
  const [rows] = await pool.query('SELECT * FROM items WHERE id = ?', [id]);
  return c.json({ rows });
});

export default app;
