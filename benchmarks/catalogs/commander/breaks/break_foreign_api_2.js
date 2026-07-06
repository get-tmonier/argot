import { z } from 'zod';

// Break: zod schema validating parsed option values — commander validates
// options via Option#argParser/choices, not a schema library; 'zod' is
// 0-usage in the corpus.
const optsSchema = z.object({
  port: z.number().int().positive(),
  host: z.string().min(1),
});

export function validateServeOptions(rawOpts) {
  return optsSchema.parse(rawOpts);
}
