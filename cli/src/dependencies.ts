import { Layer } from 'effect';
import { ExtractDatasetLive } from '#modules/extract-dataset/dependencies.ts';
import { TrainModelLive } from '#modules/train-model/dependencies.ts';
import { CalibrateLive } from '#modules/calibrate/dependencies.ts';
import { CheckVoiceLive } from '#modules/check-voice/dependencies.ts';
import { RepoContextLive } from '#modules/repo-context/dependencies.ts';

export const AppLive = Layer.mergeAll(
  ExtractDatasetLive,
  TrainModelLive,
  CalibrateLive,
  CheckVoiceLive,
  RepoContextLive,
);
