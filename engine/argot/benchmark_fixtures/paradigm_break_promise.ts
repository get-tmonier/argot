import { Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { ModelNotFound, SettingsReadError } from '#modules/repo-context/domain/errors.ts';
import type { ModelInfo, ResolvedContext } from '#modules/repo-context/domain/repo-context.ts';

export const resolveModelInfo = (
  ctx: ResolvedContext,
): Effect.Effect<ModelInfo | null, SettingsReadError, RepoContext> =>
  Effect.gen(function* () {
    const { readModelInfo } = yield* RepoContext;
    const scope = ctx.scopes[0];
    if (scope === undefined) {
      return yield* Effect.fail(new SettingsReadError({ cause: 'no scopes configured' }));
    }
    return yield* readModelInfo(scope.modelPath);
  });

export const requireModel = (
  ctx: ResolvedContext,
): Effect.Effect<ModelInfo, ModelNotFound | SettingsReadError, RepoContext> =>
  Effect.gen(function* () {
    const info = yield* resolveModelInfo(ctx);
    if (info === null) {
      return yield* Effect.fail(new ModelNotFound({ path: ctx.modelPath }));
    }
    return info;
  });

export const resolveDatasetInfo = (
  ctx: ResolvedContext,
): Effect.Effect<{ sizeBytes: number; mtime: Date } | null, SettingsReadError, RepoContext> =>
  Effect.gen(function* () {
    const { readDatasetInfo } = yield* RepoContext;
    const scope = ctx.scopes[0];
    if (scope === undefined) {
      return yield* Effect.fail(new SettingsReadError({ cause: 'no scopes configured' }));
    }
    return yield* readDatasetInfo(scope.datasetPath);
  });

// Foreign block: async/await + Promise instead of Effect.Effect
export async function fetchModelInfoAsync(
  ctx: ResolvedContext,
): Promise<{ sizeBytes: number; mtime: Date } | null> {
  const { stat } = await import('node:fs/promises');
  try {
    const s = await stat(ctx.modelPath);
    return { sizeBytes: s.size, mtime: s.mtime };
  } catch {
    return null;
  }
}

export async function requireModelAsync(ctx: ResolvedContext): Promise<ModelInfo> {
  const info = await fetchModelInfoAsync(ctx);
  if (info === null) {
    throw new Error(`Model not found at ${ctx.modelPath}. Run argot train first.`);
  }
  return info;
}

export async function fetchDatasetInfoAsync(
  ctx: ResolvedContext,
): Promise<{ sizeBytes: number; mtime: Date } | null> {
  const { stat } = await import('node:fs/promises');
  try {
    const s = await stat(ctx.datasetPath);
    return { sizeBytes: s.size, mtime: s.mtime };
  } catch {
    return null;
  }
}
