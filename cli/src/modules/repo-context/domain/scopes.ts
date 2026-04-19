import { join } from 'node:path';
import * as v from 'valibot';

export const DEFAULT_SCOPE_NAME = 'default';

export const ScopeConfigSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1)),
  path: v.string(),
});
export type ScopeConfig = v.InferOutput<typeof ScopeConfigSchema>;

export const ScopesFileSchema = v.object({
  scopes: v.array(ScopeConfigSchema),
});
export type ScopesFile = v.InferOutput<typeof ScopesFileSchema>;

export interface ResolvedScope {
  name: string;
  pathPrefix: string;
  datasetPath: string;
  modelPath: string;
}

export function resolveScopes(
  gitRoot: string,
  config: ScopesFile | undefined,
): ResolvedScope[] {
  if (!config) {
    return [
      {
        name: DEFAULT_SCOPE_NAME,
        pathPrefix: '',
        datasetPath: join(gitRoot, '.argot', 'dataset.jsonl'),
        modelPath: join(gitRoot, '.argot', 'model.pkl'),
      },
    ];
  }

  if (config.scopes.length === 0) {
    throw new Error('config.json must list at least one scope');
  }

  const seen = new Set<string>();
  for (const s of config.scopes) {
    if (seen.has(s.name)) {
      throw new Error(`duplicate scope name: ${s.name}`);
    }
    seen.add(s.name);
  }

  return config.scopes.map((s) => ({
    name: s.name,
    pathPrefix: s.path,
    datasetPath: join(gitRoot, '.argot', 'models', s.name, 'dataset.jsonl'),
    modelPath: join(gitRoot, '.argot', 'models', s.name, 'model.pkl'),
  }));
}

export function pickScope(
  scopes: readonly ResolvedScope[],
  filePath: string,
): ResolvedScope | null {
  let best: ResolvedScope | null = null;
  for (const s of scopes) {
    if (!filePath.startsWith(s.pathPrefix)) continue;
    if (best === null || s.pathPrefix.length > best.pathPrefix.length) {
      best = s;
    }
  }
  return best;
}
