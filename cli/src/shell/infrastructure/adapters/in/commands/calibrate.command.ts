import { join } from 'node:path';
import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runCalibrate } from '#modules/calibrate/application/use-cases/calibrate.use-case.ts';

export const calibrateCommand = Command.make('calibrate', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();
    yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);
    const modelAPath = join(ctx.gitRoot, '.argot', 'model_a.txt');
    const modelBPath = join(ctx.gitRoot, '.argot', 'model_b.json');
    const scorerConfigPath = join(ctx.gitRoot, '.argot', 'scorer-config.json');
    const result = yield* runCalibrate({
      repoPath: ctx.gitRoot,
      modelAPath,
      modelBPath,
      outputPath: scorerConfigPath,
      nCal: 500,
      seed: 0,
    });
    yield* Console.log(`Scorer config written to ${result.outputPath}`);
  }),
);
