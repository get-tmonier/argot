import { Layer } from 'effect';
import { ExtractDatasetLive } from '#modules/extract-dataset/dependencies.ts';

export const AppLive = Layer.mergeAll(ExtractDatasetLive);
