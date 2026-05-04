import * as v from 'valibot';

export const ExtractOptionsSchema = v.object({
  repoPath: v.string(),
  outputPath: v.string(),
  ref: v.optional(v.string()),
});

export type ExtractOptions = v.InferOutput<typeof ExtractOptionsSchema>;
