import * as v from 'valibot';

export const ExtractOptionsSchema = v.object({
  repoPath: v.string(),
  outputPath: v.string(),
});

export type ExtractOptions = v.InferOutput<typeof ExtractOptionsSchema>;
