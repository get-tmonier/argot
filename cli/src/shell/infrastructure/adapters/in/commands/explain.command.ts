import { Argument, Command, Flag } from 'effect/unstable/cli';
import { runExplain } from '#modules/explain/application/use-cases/explain.use-case.ts';

export const explainCommand = Command.make(
  'explain',
  {
    ref: Argument.string('ref').pipe(
      Argument.withDescription(
        'Git ref to explain: bare ref (HEAD, abc1234) or range (HEAD~5..HEAD)',
      ),
    ),
    model: Flag.string('model').pipe(Flag.withDefault('.argot/model.pkl')),
    dataset: Flag.string('dataset').pipe(Flag.withDefault('.argot/dataset.jsonl')),
    repo: Flag.string('repo').pipe(Flag.withDefault('.')),
    claudeModel: Flag.string('claude-model').pipe(Flag.withDefault('claude-sonnet-4-5')),
  },
  ({ ref, model, dataset, repo, claudeModel }) =>
    runExplain({ repoPath: repo, ref, modelPath: model, datasetPath: dataset, claudeModel }),
);
