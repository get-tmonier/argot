import { Effect } from 'effect';
import { Schema } from '@effect/schema';

const TrainConfigSchema = Schema.Struct({
  datasetPath: Schema.NonEmptyString,
  batchSize: Schema.Int.pipe(Schema.positive()),
  epochs: Schema.Int.pipe(Schema.positive()),
  learningRate: Schema.Number.pipe(Schema.between(0.00001, 1.0)),
});

type TrainConfig = typeof TrainConfigSchema.Type;

export const parseTrainConfig = (
  raw: unknown,
): Effect.Effect<TrainConfig, Error> =>
  Effect.gen(function* () {
    return yield* Schema.decodeUnknown(TrainConfigSchema)(raw);
  });

export const validateTrainConfig = (
  config: TrainConfig,
): Effect.Effect<TrainConfig, Error> =>
  Effect.gen(function* () {
    return yield* Schema.decodeUnknown(TrainConfigSchema)(config);
  });

export const applyTrainDefaults = (partial: Partial<TrainConfig>): TrainConfig => ({
  datasetPath: partial.datasetPath ?? '.argot/dataset.jsonl',
  batchSize: partial.batchSize ?? 32,
  epochs: partial.epochs ?? 10,
  learningRate: partial.learningRate ?? 0.001,
});

export const parseConfigManual = (raw: Record<string, unknown>): TrainConfig => {
  if (!raw['datasetPath'] || typeof raw['datasetPath'] !== 'string') {
    throw new Error('datasetPath is required and must be a non-empty string');
  }
  if (raw['batchSize'] !== undefined && typeof raw['batchSize'] !== 'number') {
    throw new Error('batchSize must be a number');
  }
  if (raw['epochs'] !== undefined) {
    if (typeof raw['epochs'] !== 'number' || raw['epochs'] <= 0) {
      throw new Error('epochs must be a positive number');
    }
  }
  if (raw['learningRate'] !== undefined) {
    if (typeof raw['learningRate'] !== 'number' || raw['learningRate'] <= 0) {
      throw new Error('learningRate must be a positive number');
    }
  }
  return {
    datasetPath: raw['datasetPath'] as string,
    batchSize: (raw['batchSize'] as number | undefined) ?? 32,
    epochs: (raw['epochs'] as number | undefined) ?? 10,
    learningRate: (raw['learningRate'] as number | undefined) ?? 0.001,
  };
};
