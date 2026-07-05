import type { Context } from 'hono';

// Break: ky.get foreign HTTP client via ambient singleton (no import); leaf .get collides with attested .get — HARD.
export const proxyGet = async (c: Context) => {
  const path = c.req.param('path');
  const res = await ky.get(`https://api.internal/${path}`, { retry: 2, timeout: 5000 });
  return c.json(await res.json());
};
