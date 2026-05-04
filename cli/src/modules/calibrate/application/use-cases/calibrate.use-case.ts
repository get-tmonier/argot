import { Effect } from 'effect';
import { EngineCalibrator } from '#modules/calibrate/application/ports/out/engine-calibrator.port.ts';
import type { CalibrateError } from '#modules/calibrate/domain/errors.ts';

interface CalibrateOptions {
  readonly repoPath: string;
  readonly repoCorpusPath: string;
  readonly genericBaselinePath: string;
  readonly outputPath: string;
  readonly nCal: number;
  readonly seed: number;
}

export const runCalibrate = (
  opts: CalibrateOptions,
): Effect.Effect<{ outputPath: string }, CalibrateError, EngineCalibrator> =>
  Effect.gen(function* () {
    const calibrator = yield* EngineCalibrator;
    yield* calibrator.runCalibrate({
      repoPath: opts.repoPath,
      repoCorpusPath: opts.repoCorpusPath,
      genericBaselinePath: opts.genericBaselinePath,
      outputPath: opts.outputPath,
      nCal: opts.nCal,
      seed: opts.seed,
    });
    return { outputPath: opts.outputPath };
  });
