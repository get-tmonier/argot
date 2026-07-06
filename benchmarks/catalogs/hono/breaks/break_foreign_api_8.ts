import type { Context } from 'hono';

// Break: jwt.sign/jwt.verify (jsonwebtoken) via ambient singleton (no import); leaves collide with attested .sign/.verify — HARD.
export const issue = async (c: Context) => {
  const body = await c.req.json<{ sub: string }>();
  const token = jwt.sign({ sub: body.sub }, 'secret', { expiresIn: '1h', algorithm: 'HS256' });
  const decoded = jwt.verify(token, 'secret');
  return c.json({ token, decoded });
};
