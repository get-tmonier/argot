import { Effect } from 'effect';
import { TrainingError } from '#modules/train-model/domain/errors.ts';
import { ModelTrainer } from '#modules/train-model/application/ports/out/model-trainer.port.ts';
import type { TrainOptions } from '#modules/train-model/domain/train-options.ts';

export const runTrainModel = (
  opts: TrainOptions,
): Effect.Effect<{ modelPath: string }, TrainingError, ModelTrainer> =>
  Effect.gen(function* () {
    const trainer = yield* ModelTrainer;
    if (opts.epochs <= 0) {
      return yield* Effect.fail(new TrainingError({ reason: 'epochs must be positive' }));
    }
    if (opts.batchSize <= 0) {
      return yield* Effect.fail(new TrainingError({ reason: 'batch size must be positive' }));
    }
    yield* trainer.runTrain(opts);
    return { modelPath: opts.modelPath };
  });

export const validateTrainOptions = (
  opts: TrainOptions,
): Effect.Effect<TrainOptions, TrainingError> =>
  Effect.gen(function* () {
    if (opts.datasetPath === '') {
      return yield* Effect.fail(new TrainingError({ reason: 'dataset path required' }));
    }
    if (opts.learningRate <= 0) {
      return yield* Effect.fail(new TrainingError({ reason: 'learning rate must be positive' }));
    }
    return opts;
  });

export const assertPositive = (value: number, name: string): number => {
  if (value <= 0) {
    throw new Error(`${name} must be positive, got ${value}`);
  }
  return value;
};
