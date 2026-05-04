import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import { EngineExitNonZero, EngineSpawnFailed } from '#modules/extract-dataset/domain/errors.ts';

export const BunEngineRunnerLive = Layer.effect(EngineRunner)(
  Effect.succeed({
    runExtract: ({
      repoPath,
      outputPath,
      ref,
    }: {
      repoPath: string;
      outputPath: string;
      ref?: string;
    }) =>
      Effect.callback<void, EngineExitNonZero | EngineSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.extract');

        // Pass ref as second positional when non-empty
        const positionals = ref !== undefined && ref.length > 0 ? [repoPath, ref] : [repoPath];

        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(cmd, [...args, ...positionals, '--out', outputPath], {
            stdio: ['ignore', 'inherit', 'pipe'],
          });
        } catch (cause: unknown) {
          resume(Effect.fail(new EngineSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new EngineSpawnFailed({ cause })));
        });

        proc.on('close', (code: number | null) => {
          stopSpinner();
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
