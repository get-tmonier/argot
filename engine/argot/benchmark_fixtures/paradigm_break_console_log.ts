import { Effect, Console } from 'effect';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import type { ExtractOptions } from '#modules/extract-dataset/domain/extract-options.ts';
import type { EngineError } from '#modules/extract-dataset/domain/errors.ts';

export const runExtractDataset = (
  opts: ExtractOptions,
): Effect.Effect<{ outputPath: string }, EngineError, EngineRunner> =>
  Effect.gen(function* () {
    const engineRunner = yield* EngineRunner;
    yield* Console.log(`extracting ${opts.repoPath}`);
    yield* engineRunner.runExtract({ repoPath: opts.repoPath, outputPath: opts.outputPath });
    yield* Effect.logInfo(`wrote dataset to ${opts.outputPath}`);
    return { outputPath: opts.outputPath };
  });

export const runExtractWithProgress = (
  opts: ExtractOptions,
): Effect.Effect<{ outputPath: string; durationMs: number }, EngineError, EngineRunner> =>
  Effect.gen(function* () {
    const engineRunner = yield* EngineRunner;
    const start = yield* Effect.sync(() => Date.now());
    yield* Effect.logInfo('starting extract pipeline');
    yield* engineRunner.runExtract({ repoPath: opts.repoPath, outputPath: opts.outputPath });
    const durationMs = yield* Effect.sync(() => Date.now() - start);
    yield* Effect.logInfo(`extract finished in ${durationMs}ms`);
    return { outputPath: opts.outputPath, durationMs };
  });

export const reportExtractFailure = (
  err: EngineError,
): Effect.Effect<void, never> =>
  Effect.gen(function* () {
    yield* Effect.logError('extract failed', err);
    console.log('DEBUG: extract error payload', err);
    yield* Effect.logWarning('falling back to cached dataset if available');
  });
