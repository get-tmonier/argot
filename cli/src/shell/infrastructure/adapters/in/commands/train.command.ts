import { Command, Flag } from 'effect/unstable/cli';
import { Console, Effect, Option } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { ScopeNotFound } from '#modules/repo-context/domain/errors.ts';
import { runTrainModel } from '#modules/train-model/application/use-cases/train-model.use-case.ts';

export const trainCommand = Command.make(
  'train',
  {
    scope: Flag.string('scope').pipe(
      Flag.optional,
      Flag.withDescription('Only train a single scope by name (multi-scope configs only)'),
    ),
  },
  ({ scope }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);

      const scopeName = Option.getOrNull(scope);
      const scopesToRun = scopeName
        ? ctx.scopes.filter((s) => s.name === scopeName)
        : ctx.scopes;

      if (scopeName && scopesToRun.length === 0) {
        return yield* Effect.fail(
          new ScopeNotFound({ name: scopeName, available: ctx.scopes.map((s) => s.name) }),
        );
      }

      for (const s of scopesToRun) {
        yield* Console.log(`→ scope ${s.name}`);
        yield* runTrainModel({ datasetPath: s.datasetPath, modelPath: s.modelPath });
        yield* Console.log(`  model → ${s.modelPath}`);
      }
    }),
);
