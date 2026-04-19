import { Command, Flag } from 'effect/unstable/cli';
import { Console, Effect, Option } from 'effect';
import { mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { ScopeNotFound } from '#modules/repo-context/domain/errors.ts';
import { runExtractDataset } from '#modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts';

export const extractCommand = Command.make(
  'extract',
  {
    scope: Flag.string('scope').pipe(
      Flag.optional,
      Flag.withDescription('Only extract a single scope by name (multi-scope configs only)'),
    ),
  },
  ({ scope }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);

      const scopeName = Option.getOrNull(scope);
      const scopesToRun = scopeName ? ctx.scopes.filter((s) => s.name === scopeName) : ctx.scopes;

      if (scopeName && scopesToRun.length === 0) {
        return yield* Effect.fail(
          new ScopeNotFound({ name: scopeName, available: ctx.scopes.map((s) => s.name) }),
        );
      }

      for (const s of scopesToRun) {
        yield* Console.log(`→ scope ${s.name}${s.pathPrefix ? ` (${s.pathPrefix})` : ''}`);
        yield* Effect.tryPromise(() => mkdir(dirname(s.datasetPath), { recursive: true }));
        const result = yield* runExtractDataset({
          repoPath: ctx.gitRoot,
          outputPath: s.datasetPath,
          pathPrefix: s.pathPrefix === '' ? undefined : s.pathPrefix,
        });
        yield* Console.log(`  dataset → ${result.outputPath}`);
      }
    }),
);
