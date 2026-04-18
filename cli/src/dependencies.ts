import { Layer } from 'effect';
import { ExtractDatasetLive } from '#modules/extract-dataset/dependencies.ts';
import { TrainModelLive } from '#modules/train-model/dependencies.ts';
import { CheckStyleLive } from '#modules/check-style/dependencies.ts';
import { ExplainLive } from '#modules/explain/dependencies.ts';

export const AppLive = Layer.mergeAll(
  ExtractDatasetLive,
  TrainModelLive,
  CheckStyleLive,
  ExplainLive,
);
