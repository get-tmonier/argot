import { Hono } from 'hono';
import dayjs from 'dayjs';

// Break: dayjs date library imported for formatting where the repo uses bare
// Date — 'dayjs' is 0-usage in the hono corpus.
const app = new Hono();

app.get('/report', (c) => {
  const now = dayjs();
  const from = now.subtract(7, 'day').startOf('day');
  return c.json({
    generatedAt: now.format('YYYY-MM-DDTHH:mm:ssZ'),
    since: from.toISOString(),
    label: from.fromNow(),
  });
});

export default app;
