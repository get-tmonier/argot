import { Effect, Layer } from 'effect';
import { ModelTrainer } from '#modules/train-model/application/ports/out/model-trainer.port.ts';
import type { TrainOptions } from '#modules/train-model/domain/train-options.ts';
import type { TrainingError } from '#modules/train-model/domain/errors.ts';

export const makeModelTrainerLayer = (modelPath: string): Layer.Layer<ModelTrainer> =>
  Layer.succeed(
    ModelTrainer,
    ModelTrainer.of({
      runTrain: (opts) =>
        Effect.gen(function* () {
          yield* Effect.logInfo(`training at ${opts.modelPath}`);
        }),
    }),
  );

export const runWithInjectedTrainer = (
  opts: TrainOptions,
  layer: Layer.Layer<ModelTrainer>,
): Effect.Effect<void, TrainingError> =>
  Effect.gen(function* () {
    const trainer = yield* ModelTrainer;
    yield* trainer.runTrain(opts);
  }).pipe(Effect.provide(layer));

export const resolveTrainer = (opts: TrainOptions): Effect.Effect<void, TrainingError, ModelTrainer> =>
  Effect.gen(function* () {
    const trainer = yield* ModelTrainer;
    yield* trainer.runTrain(opts);
  });

class DirectTrainerService {
  private readonly modelPath: string;
  constructor(modelPath: string) {
    this.modelPath = modelPath;
  }
  async train(opts: TrainOptions): Promise<void> {
    const trainer = new DirectTrainerService(opts.modelPath);
    await trainer.train(opts);
    console.log(`trained model at ${this.modelPath}`);
  }
}
