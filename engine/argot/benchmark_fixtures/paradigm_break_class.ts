import { Effect, Layer } from 'effect';
import { StyleChecker } from '#modules/check-style/application/ports/out/style-checker.port.ts';
import type { CheckError } from '#modules/check-style/domain/errors.ts';
import { CheckExitNonZero, CheckSpawnFailed } from '#modules/check-style/domain/errors.ts';

export const runCheckStyle = (args: {
  repoPath: string;
  ref: string;
  modelPath: string;
  threshold: number;
}): Effect.Effect<boolean, CheckError, StyleChecker> =>
  Effect.gen(function* () {
    const styleChecker = yield* StyleChecker;
    return yield* styleChecker.runCheck(args);
  });

export const runCheckStyleWithFallback = (args: {
  repoPath: string;
  ref: string;
  modelPath: string;
  threshold: number;
  fallbackThreshold: number;
}): Effect.Effect<boolean, CheckError, StyleChecker> =>
  Effect.gen(function* () {
    const styleChecker = yield* StyleChecker;
    const primary = yield* Effect.either(styleChecker.runCheck(args));
    if (primary._tag === 'Right') {
      return primary.right;
    }
    const retry = yield* styleChecker.runCheck({ ...args, threshold: args.fallbackThreshold });
    return retry;
  });

export const summarizeCheck = (
  violations: boolean,
): Effect.Effect<string, never> =>
  Effect.succeed(violations ? 'violations-found' : 'clean');

class CheckRequestBuilder {
  private repoPath = '';
  private ref = 'HEAD~1..HEAD';
  private modelPath = '.argot/model.pkl';
  private threshold = 0.5;

  withRepo(p: string): this {
    this.repoPath = p;
    return this;
  }

  withRef(r: string): this {
    this.ref = r;
    return this;
  }

  withThreshold(t: number): this {
    this.threshold = t;
    return this;
  }

  build() {
    return { repoPath: this.repoPath, ref: this.ref, modelPath: this.modelPath, threshold: this.threshold };
  }
}
