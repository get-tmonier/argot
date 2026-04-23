import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runTrainModel } from '#modules/train-model/application/use-cases/train-model.use-case.ts';

export const trainCommand = Command.make('train', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();
    yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);
    yield* runTrainModel({
      repoPath: ctx.gitRoot,
      modelAPath: ctx.modelAPath,
      modelBPath: ctx.modelBPath,
    });
    yield* Console.log(`Model written to ${ctx.argotDir}`);
  }),
);
