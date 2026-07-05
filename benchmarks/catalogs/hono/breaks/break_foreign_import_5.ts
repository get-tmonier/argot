import { Hono } from 'hono';
import { formatISO as isoStamp, subDays as minusDays } from 'date-fns';

// Break: date-fns aliased import (formatISO/subDays) for report windows (repo uses bare Date) — 0-usage.
const app = new Hono();

app.get('/report', (c) => {
  const now = new Date();
  const since = minusDays(now, 7);
  return c.json({ generatedAt: isoStamp(now), since: isoStamp(since) });
});

export default app;
