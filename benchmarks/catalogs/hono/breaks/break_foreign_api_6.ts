import type { Context } from 'hono';

// Break: mongoose.model + new mongoose.Schema foreign namespace (no import) in a Hono handler — 0-usage.
const User = mongoose.model(
  'User',
  new mongoose.Schema({ email: String, createdAt: Date })
);

export const getUser = async (c: Context) => {
  const id = c.req.param('id');
  const user = await User.findById(id);
  return c.json({ user });
};
