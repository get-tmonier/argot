import { Hono } from 'hono';
import { S3Client, PutObjectCommand } from '@aws-sdk/client-s3';

// Break: AWS S3 SDK S3Client + PutObjectCommand in a Hono handler — 0-usage.
const app = new Hono();

const s3 = new S3Client({ region: 'us-east-1' });

app.put('/upload/:key', async (c) => {
  const key = c.req.param('key');
  const body = await c.req.arrayBuffer();
  await s3.send(
    new PutObjectCommand({ Bucket: 'assets', Key: key, Body: new Uint8Array(body) })
  );
  return c.json({ stored: key });
});

export default app;
