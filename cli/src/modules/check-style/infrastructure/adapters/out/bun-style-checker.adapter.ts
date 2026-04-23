import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { StyleChecker } from '#modules/check-style/application/ports/out/style-checker.port.ts';
import { CheckExitNonZero, CheckSpawnFailed } from '#modules/check-style/domain/errors.ts';

export const BunStyleCheckerLive = Layer.effect(StyleChecker)(
  Effect.succeed({
    runCheck: ({
      repoPath,
      ref,
      argotDir,
      threshold,
    }: {
      repoPath: string;
      ref: string;
      argotDir: string;
      threshold: number;
    }) =>
      Effect.callback<boolean, CheckExitNonZero | CheckSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.check');
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(
            cmd,
            [...args, repoPath, ref, '--argot-dir', argotDir, '--threshold', String(threshold)],
            { stdio: ['ignore', 'inherit', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new CheckSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new CheckSpawnFailed({ cause })));
        });
        proc.on('close', (code: number | null) => {
          stopSpinner();
          if (code === 0) {
            resume(Effect.succeed(false));
          } else if (code === 1) {
            resume(Effect.succeed(true));
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new CheckExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
