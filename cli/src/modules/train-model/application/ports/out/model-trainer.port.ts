import { Context } from 'effect';
import type { Effect } from 'effect';
import type { TrainError } from '#modules/train-model/domain/errors.ts';

interface ModelTrainerShape {
  readonly runTrain: (args: {
    datasetPath: string;
    modelPath: string;
  }) => Effect.Effect<void, TrainError>;
}

export class ModelTrainer extends Context.Service<ModelTrainer, ModelTrainerShape>()(
  '@argot/ModelTrainer',
) {}
