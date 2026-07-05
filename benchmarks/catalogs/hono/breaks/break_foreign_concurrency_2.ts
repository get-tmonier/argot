import { Hono } from 'hono';
import { Queue } from 'bullmq';

// Break: BullMQ background job queue enqueued from a Hono handler where the
// repo has no worker/queue runtime — 'bullmq'/new Queue are 0-usage in corpus.
const app = new Hono();

const emailQueue = new Queue('emails', {
  connection: { host: '127.0.0.1', port: 6379 },
});

app.post('/notify', async (c) => {
  const body = await c.req.json<{ to: string; subject: string }>();
  const job = await emailQueue.add('send', {
    to: body.to,
    subject: body.subject,
  });
  return c.json({ enqueued: true, jobId: job.id });
});

export default app;
