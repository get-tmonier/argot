import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { EngineCalibrator } from '#modules/calibrate/application/ports/out/engine-calibrator.port.ts';
import { CalibrateExitNonZero, CalibrateSpawnFailed } from '#modules/calibrate/domain/errors.ts';

export const BunEngineCalibratorLive = Layer.effect(EngineCalibrator)(
  Effect.succeed({
    runCalibrate: ({
      repoPath,
      modelAPath,
      modelBPath,
      outputPath,
      nCal,
      seed,
    }: {
      repoPath: string;
      modelAPath: string;
      modelBPath: string;
      outputPath: string;
      nCal: number;
      seed: number;
    }) =>
      Effect.callback<void, CalibrateExitNonZero | CalibrateSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.scoring.calibration');
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(
            cmd,
            [
              ...args,
              '--repo',
              repoPath,
              '--model-a',
              modelAPath,
              '--model-b',
              modelBPath,
              '--output',
              outputPath,
              '--n-cal',
              String(nCal),
              '--seed',
              String(seed),
            ],
            { stdio: ['ignore', 'inherit', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new CalibrateSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new CalibrateSpawnFailed({ cause })));
        });
        proc.on('close', (code: number | null) => {
          stopSpinner();
          if (code === 0) {
            resume(Effect.void);
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new CalibrateExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
