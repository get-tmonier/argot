import { Effect } from 'effect';
import { StyleChecker } from '#modules/check-style/application/ports/out/style-checker.port.ts';
import type { CheckError } from '#modules/check-style/domain/errors.ts';

export const runCheckStyle = (args: {
  repoPath: string;
  ref: string;
  modelPath: string;
  threshold: number;
}): Effect.Effect<void, CheckError, StyleChecker> =>
  Effect.gen(function* () {
    const styleChecker = yield* StyleChecker;
    yield* styleChecker.runCheck(args);
  });
