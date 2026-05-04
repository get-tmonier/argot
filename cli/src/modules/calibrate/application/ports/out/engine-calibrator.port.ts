import { Context } from 'effect';
import type { Effect } from 'effect';
import type { CalibrateError } from '#modules/calibrate/domain/errors.ts';

interface EngineCalibratorShape {
  readonly runCalibrate: (args: {
    repoPath: string;
    repoCorpusPath: string;
    genericBaselinePath: string;
    outputPath: string;
    nCal: number;
    seed: number;
  }) => Effect.Effect<void, CalibrateError>;
}

export class EngineCalibrator extends Context.Service<EngineCalibrator, EngineCalibratorShape>()(
  '@argot/EngineCalibrator',
) {}
