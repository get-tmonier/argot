import { Hono } from 'hono';
import bodyParser from 'body-parser';
import cors from 'cors';

const app = new Hono();

// Break: Express middleware chain (bodyParser, cors) wired via app.use.
app.use(bodyParser.json());
app.use(bodyParser.urlencoded({ extended: true }));
app.use(cors({ origin: '*' }));

app.post('/submit', (c) => {
  return c.json({ received: true });
});

export default app;
