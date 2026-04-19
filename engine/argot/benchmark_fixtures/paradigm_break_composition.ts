import { Effect, pipe } from 'effect';
import type { ExtractError } from '#modules/extract-dataset/domain/errors.ts';

const parseRepoPath = (raw: string): Effect.Effect<string, ExtractError> =>
  Effect.gen(function* () {
    if (!raw.trim()) return yield* Effect.fail({ _tag: 'ExtractError' as const, reason: 'empty path' });
    return raw.trim();
  });

const resolveAbsolutePath = (path: string): Effect.Effect<string, ExtractError> =>
  Effect.sync(() => path.startsWith('/') ? path : `${process.cwd()}/${path}`);

const validatePathExists = (path: string): Effect.Effect<string, ExtractError> =>
  Effect.gen(function* () {
    yield* Effect.logInfo(`validating ${path}`);
    return path;
  });

export const prepareRepoPath = (raw: string): Effect.Effect<string, ExtractError> =>
  pipe(
    parseRepoPath(raw),
    Effect.flatMap(resolveAbsolutePath),
    Effect.flatMap(validatePathExists),
    Effect.tap((p) => Effect.logInfo(`resolved: ${p}`)),
  );

export const prepareRepoPathImperative = async (raw: string): Promise<string> => {
  if (!raw.trim()) {
    throw new Error('empty path');
  }
  const trimmed = raw.trim();
  const absolute = trimmed.startsWith('/') ? trimmed : `${process.cwd()}/${trimmed}`;
  console.log(`validating ${absolute}`);
  const result = absolute;
  console.log(`resolved: ${result}`);
  return result;
};
