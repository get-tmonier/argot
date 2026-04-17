import { Effect, Layer } from 'effect';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import { EngineExitNonZero, EngineSpawnFailed } from '#modules/extract-dataset/domain/errors.ts';

export const BunEngineRunnerLive = Layer.effect(
  EngineRunner,
  Effect.succeed({
    runExtract: ({ repoPath, outputPath }: { repoPath: string; outputPath: string }) =>
      Effect.async<void, EngineExitNonZero | EngineSpawnFailed>((resume) => {
        let proc: ReturnType<typeof Bun.spawn>;
        try {
          proc = Bun.spawn(
            ['uv', 'run', '--package', 'argot-engine', 'python', '-m', 'argot.extract', repoPath, '--out', outputPath],
            { stdout: 'inherit', stderr: 'pipe' },
          );
        } catch (cause) {
          resume(Effect.fail(new EngineSpawnFailed({ cause })));
          return;
        }

        proc.exited.then(async (exitCode) => {
          if (exitCode === 0) {
            resume(Effect.void);
          } else {
            const stderrText = await new Response(proc.stderr).text();
            resume(Effect.fail(new EngineExitNonZero({ exitCode, stderr: stderrText })));
          }
        });
      }),
  }),
);
