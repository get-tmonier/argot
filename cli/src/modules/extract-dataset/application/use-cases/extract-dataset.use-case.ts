import { Effect } from 'effect';
import type { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import type { ExtractOptions } from '#modules/extract-dataset/domain/extract-options.ts';
import type { EngineError } from '#modules/extract-dataset/domain/errors.ts';

export function createExtractDatasetUseCase(deps: { engineRunner: EngineRunner }) {
  return {
    run(opts: ExtractOptions): Effect.Effect<{ outputPath: string }, EngineError> {
      return Effect.gen(function* () {
        yield* deps.engineRunner.runExtract({
          repoPath: opts.repoPath,
          outputPath: opts.outputPath,
        });
        return { outputPath: opts.outputPath };
      });
    },
  };
}
