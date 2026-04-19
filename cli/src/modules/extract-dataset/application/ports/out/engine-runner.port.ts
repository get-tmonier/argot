import { Context } from 'effect';
import type { Effect } from 'effect';
import type { EngineError } from '#modules/extract-dataset/domain/errors.ts';

interface EngineRunnerShape {
  readonly runExtract: (args: {
    repoPath: string;
    outputPath: string;
    pathPrefix?: string;
  }) => Effect.Effect<void, EngineError>;
}

export class EngineRunner extends Context.Service<EngineRunner, EngineRunnerShape>()(
  '@argot/EngineRunner',
) {}
