import Koa from 'koa';
import Router from '@koa/router';

// Break: Koa app with ctx middleware pattern in a Hono project.
// Import at line 1 is the dead giveaway.
const app = new Koa();
const router = new Router();

router.get('/items', async (ctx) => {
  ctx.body = { items: [] };
});

router.post('/items', async (ctx) => {
  ctx.status = 201;
  ctx.body = { created: true };
});

app.use(router.routes());
app.use(router.allowedMethods());

export default app;
