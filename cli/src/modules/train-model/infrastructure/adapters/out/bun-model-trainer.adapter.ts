import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { ModelTrainer } from '#modules/train-model/application/ports/out/model-trainer.port.ts';
import { TrainExitNonZero, TrainSpawnFailed } from '#modules/train-model/domain/errors.ts';

export const BunModelTrainerLive = Layer.effect(ModelTrainer)(
  Effect.succeed({
    runTrain: ({
      repoPath,
      modelAPath,
      modelBPath,
    }: {
      repoPath: string;
      modelAPath: string;
      modelBPath: string;
    }) =>
      Effect.callback<void, TrainExitNonZero | TrainSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.train');
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(
            cmd,
            [...args, '--repo', repoPath, '--model-a-out', modelAPath, '--model-b-out', modelBPath],
            { stdio: ['ignore', 'inherit', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new TrainSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new TrainSpawnFailed({ cause })));
        });
        proc.on('close', (code: number | null) => {
          stopSpinner();
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
