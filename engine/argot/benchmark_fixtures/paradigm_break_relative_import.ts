import { Effect, Layer } from 'effect';
import { CheckCommand } from '#shell/infrastructure/adapters/in/commands/check.command.ts';
import { BunStyleCheckerLive } from '#modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts';
import { FsRepoContextLive } from '#modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts';
import { StyleChecker } from '#modules/check-style/application/ports/out/style-checker.port.ts';
import { RepoContext } from '#modules/repo-context/application/ports/out/repo-context.port.ts';
import { ExtractCommand } from '#shell/infrastructure/adapters/in/commands/extract.command.ts';
import { TrainCommand } from '#shell/infrastructure/adapters/in/commands/train.command.ts';
import { ExplainCommand } from '#shell/infrastructure/adapters/in/commands/explain.command.ts';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import { ModelTrainer } from '#modules/train-model/application/ports/out/model-trainer.port.ts';
import { Explainer } from '#modules/explain/application/ports/out/explainer.port.ts';
import { BunEngineRunnerLive } from '#modules/extract-dataset/infrastructure/adapters/out/bun-engine-runner.adapter.ts';
import { BunModelTrainerLive } from '#modules/train-model/infrastructure/adapters/out/bun-model-trainer.adapter.ts';
import { BunExplainerLive } from '../../../legacy/explainer/bun-explainer.adapter.ts';

export const AppLive = Layer.mergeAll(
  BunStyleCheckerLive,
  FsRepoContextLive,
  BunEngineRunnerLive,
  BunModelTrainerLive,
  BunExplainerLive,
);

export const wireCommands = {
  check: CheckCommand,
  extract: ExtractCommand,
  train: TrainCommand,
  explain: ExplainCommand,
};
