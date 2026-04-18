import { Effect } from 'effect';
import { Explainer } from '#modules/explain/application/ports/out/explainer.port.ts';
import type { ExplainOptions } from '#modules/explain/domain/explain-options.ts';
import type { ExplainError } from '#modules/explain/domain/errors.ts';

export const runExplain = (
  opts: ExplainOptions,
): Effect.Effect<ReadonlyArray<string>, ExplainError, Explainer> =>
  Effect.gen(function* () {
    const explainer = yield* Explainer;
    const anomalies = yield* explainer.runExplain(opts);
    return anomalies.map((a) => a.hunkText);
  });

export const runExplainWithContext = (
  opts: ExplainOptions,
): Effect.Effect<ReadonlyArray<{ hunk: string; context: string }>, ExplainError, Explainer> =>
  Effect.gen(function* () {
    const explainer = yield* Explainer;
    const anomalies = yield* explainer.runExplain(opts);
    return anomalies.map((a) => ({ hunk: a.hunkText, context: a.contextText }));
  });

export const countExplainAnomalies = (
  opts: ExplainOptions,
): Effect.Effect<number, ExplainError, Explainer> =>
  Effect.gen(function* () {
    const explainer = yield* Explainer;
    const anomalies = yield* explainer.runExplain(opts);
    return anomalies.length;
  });
