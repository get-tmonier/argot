import { Argument, Command, Flag } from 'effect/unstable/cli';
import { runExplain } from '#modules/explain/application/use-cases/explain.use-case.ts';

export const explainCommand = Command.make(
  'explain',
  {
    ref: Argument.string('ref'),
    model: Flag.string('model').pipe(Flag.withDefault('.argot/model.pkl')),
    dataset: Flag.string('dataset').pipe(Flag.withDefault('.argot/dataset.jsonl')),
  },
  ({ ref, model, dataset }) =>
    runExplain({ repoPath: '.', ref, modelPath: model, datasetPath: dataset }),
);
