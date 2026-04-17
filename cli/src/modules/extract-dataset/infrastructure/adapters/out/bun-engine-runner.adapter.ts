import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import { EngineExitNonZero, EngineSpawnFailed } from '#modules/extract-dataset/domain/errors.ts';

export const BunEngineRunnerLive = Layer.effect(EngineRunner)(
  Effect.succeed({
    runExtract: ({ repoPath, outputPath }: { repoPath: string; outputPath: string }) =>
      Effect.callback<void, EngineExitNonZero | EngineSpawnFailed>((resume) => {
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
              'argot.extract',
              repoPath,
              '--out',
              outputPath,
            ],
            { stdio: ['ignore', 'inherit', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new EngineSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        proc.stderr!.on('data', (chunk: Buffer) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new EngineSpawnFailed({ cause })));
        });

        proc.on('close', (code: number | null) => {
          if (code === 0) {
            resume(Effect.void);
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new EngineExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
