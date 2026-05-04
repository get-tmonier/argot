import { join } from 'node:path';
import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runTrainModel } from '#modules/train-model/application/use-cases/train-model.use-case.ts';
import { runCalibrate } from '#modules/calibrate/application/use-cases/calibrate.use-case.ts';
import { brandedArgot } from '#branding.ts';

/**
 * `argot fit` — one-shot voice fitting: builds the repo corpus + generic
 * baseline (train), then samples calibration hunks to set the threshold
 * (calibrate). The two underlying use-cases stay separate in code so
 * benchmarks and research scripts can still call them independently; the
 * CLI surface collapses them into one step matching the user's mental model
 * ("fit the voice model to this repo").
 */
export const fitCommand = Command.make('fit', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();
    yield* Console.log(`${brandedArgot()} · ${ctx.name} (${ctx.gitRoot})`);

    yield* Console.log('Step 1/2: training voice model …');
    yield* runTrainModel({
      repoPath: ctx.gitRoot,
      repoCorpusPath: ctx.repoCorpusPath,
      genericBaselinePath: ctx.genericBaselinePath,
    });

    yield* Console.log('Step 2/2: calibrating threshold …');
    const scorerConfigPath = join(ctx.argotDir, 'scorer-config.json');
    const result = yield* runCalibrate({
      repoPath: ctx.gitRoot,
      repoCorpusPath: ctx.repoCorpusPath,
      genericBaselinePath: ctx.genericBaselinePath,
      outputPath: scorerConfigPath,
      nCal: 500,
      seed: 0,
    });

    yield* Console.log(`Done. Scorer config: ${result.outputPath}`);
  }),
);
