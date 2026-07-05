import type { Context, Next } from 'hono';

// Break: pino() foreign logger factory (no import) in Hono middleware — 0-usage.
const log = pino({ level: 'info', name: 'http' });

export const audit = async (c: Context, next: Next) => {
  log.info({ method: c.req.method, path: c.req.path }, 'request');
  await next();
};
