import { Context } from 'effect';
import type { Effect } from 'effect';
import type { ExplainError } from '#modules/explain/domain/errors.ts';

interface ExplainerShape {
  readonly runExplain: (args: {
    repoPath: string;
    ref: string;
    modelPath: string;
    datasetPath: string;
    claudeModel: string;
  }) => Effect.Effect<void, ExplainError>;
}

export class Explainer extends Context.Service<Explainer, ExplainerShape>()('@argot/Explainer') {}
