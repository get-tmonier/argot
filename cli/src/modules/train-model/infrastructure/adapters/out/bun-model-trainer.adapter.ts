import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { ModelTrainer } from '#modules/train-model/application/ports/out/model-trainer.port.ts';
import { TrainExitNonZero, TrainSpawnFailed } from '#modules/train-model/domain/errors.ts';

export const BunModelTrainerLive = Layer.effect(ModelTrainer)(
  Effect.succeed({
    runTrain: ({ datasetPath, modelPath }: { datasetPath: string; modelPath: string }) =>
      Effect.callback<void, TrainExitNonZero | TrainSpawnFailed>((resume) => {
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(
            'uv',
            [
              'run',
              '--package',
              'argot-engine',
              'python',
              '-m',
              'argot.train',
              '--dataset',
              datasetPath,
              '--out',
              modelPath,
            ],
            { stdio: ['ignore', 'inherit', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new TrainSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        proc.stderr!.on('data', (chunk: Buffer) => stderrChunks.push(chunk));
        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new TrainSpawnFailed({ cause })));
        });
        proc.on('close', (code: number | null) => {
          if (code === 0) {
            resume(Effect.void);
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new TrainExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
