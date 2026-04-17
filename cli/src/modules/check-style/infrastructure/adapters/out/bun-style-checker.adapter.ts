import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { StyleChecker } from '#modules/check-style/application/ports/out/style-checker.port.ts';
import { CheckExitNonZero, CheckSpawnFailed } from '#modules/check-style/domain/errors.ts';

export const BunStyleCheckerLive = Layer.effect(StyleChecker)(
  Effect.succeed({
    runCheck: ({
      repoPath,
      ref,
      modelPath,
    }: {
      repoPath: string;
      ref: string;
      modelPath: string;
    }) =>
      Effect.callback<void, CheckExitNonZero | CheckSpawnFailed>((resume) => {
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
              'argot.check',
              repoPath,
              ref,
              '--model',
              modelPath,
            ],
            { stdio: ['ignore', 'inherit', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new CheckSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        proc.stderr!.on('data', (chunk: Buffer) => stderrChunks.push(chunk));
        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new CheckSpawnFailed({ cause })));
        });
        proc.on('close', (code: number | null) => {
          if (code === 0) {
            resume(Effect.void);
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new CheckExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
