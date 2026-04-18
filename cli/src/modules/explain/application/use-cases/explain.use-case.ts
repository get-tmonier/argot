import { Effect } from 'effect';
import { Explainer } from '#modules/explain/application/ports/out/explainer.port.ts';
import type { ExplainError } from '#modules/explain/domain/errors.ts';

export const runExplain = (args: {
  repoPath: string;
  ref: string;
  modelPath: string;
  datasetPath: string;
}): Effect.Effect<void, ExplainError, Explainer> =>
  Effect.gen(function* () {
    const explainer = yield* Explainer;
    yield* explainer.runExplain(args);
  });
