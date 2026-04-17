import { Effect } from 'effect';
import { ModelTrainer } from '#modules/train-model/application/ports/out/model-trainer.port.ts';
import type { TrainError } from '#modules/train-model/domain/errors.ts';

export const runTrainModel = (args: {
  datasetPath: string;
  modelPath: string;
}): Effect.Effect<void, TrainError, ModelTrainer> =>
  Effect.gen(function* () {
    const modelTrainer = yield* ModelTrainer;
    yield* modelTrainer.runTrain({ datasetPath: args.datasetPath, modelPath: args.modelPath });
  });
