import { Command, Flag } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { runTrainModel } from '#modules/train-model/application/use-cases/train-model.use-case.ts';

export const trainCommand = Command.make(
  'train',
  {
    dataset: Flag.string('dataset').pipe(Flag.withDefault('.argot/dataset.jsonl')),
    model: Flag.string('model').pipe(Flag.withDefault('.argot/model.pkl')),
  },
  ({ dataset, model }) =>
    Effect.gen(function* () {
      yield* runTrainModel({ datasetPath: dataset, modelPath: model });
      yield* Console.log(`Model written to ${model}`);
    }),
);
