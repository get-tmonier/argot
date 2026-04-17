import { Effect } from 'effect';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import type { ExtractOptions } from '#modules/extract-dataset/domain/extract-options.ts';
import type { EngineError } from '#modules/extract-dataset/domain/errors.ts';

export const runExtractDataset = (
  opts: ExtractOptions,
): Effect.Effect<{ outputPath: string }, EngineError, EngineRunner> =>
  Effect.gen(function* () {
    const engineRunner = yield* EngineRunner;
    yield* engineRunner.runExtract({ repoPath: opts.repoPath, outputPath: opts.outputPath });
    return { outputPath: opts.outputPath };
  });
