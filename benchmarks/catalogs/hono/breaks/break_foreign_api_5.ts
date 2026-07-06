import type { Context, Next } from 'hono';

// Break: winston.createLogger + winston.transports foreign namespace (no import) in Hono middleware — 0-usage.
const logger = winston.createLogger({
  level: 'info',
  transports: [new winston.transports.Console()],
});

export const requestLogger = async (c: Context, next: Next) => {
  logger.info(`${c.req.method} ${c.req.path}`);
  await next();
};
