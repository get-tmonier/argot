import { Hono } from 'hono';
import { PrismaClient } from '@prisma/client';

// Break: Prisma ORM client instantiated in a Hono handler where the repo has
// no database layer — '@prisma/client'/PrismaClient are 0-usage in the corpus.
const app = new Hono();

const prisma = new PrismaClient();

app.get('/users/:id', async (c) => {
  const id = c.req.param('id');
  const user = await prisma.user.findUnique({
    where: { id: Number(id) },
    include: { posts: true },
  });
  if (!user) {
    return c.json({ error: 'not found' }, 404);
  }
  return c.json(user);
});

export default app;
