import { ServiceMap } from 'effect';
import type { Effect } from 'effect';
import type { EngineError } from '#modules/extract-dataset/domain/errors.ts';

export interface EngineRunnerShape {
  readonly runExtract: (args: {
    repoPath: string;
    outputPath: string;
  }) => Effect.Effect<void, EngineError>;
}

export class EngineRunner extends ServiceMap.Service<EngineRunner>()(
  '@argot/EngineRunner',
)<EngineRunnerShape> {}
