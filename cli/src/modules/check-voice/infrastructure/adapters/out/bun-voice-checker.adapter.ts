import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import {
  VoiceChecker,
  type MinSeverity,
} from '#modules/check-voice/application/ports/out/voice-checker.port.ts';
import { CheckExitNonZero, CheckSpawnFailed } from '#modules/check-voice/domain/errors.ts';

export const BunVoiceCheckerLive = Layer.effect(VoiceChecker)(
  Effect.succeed({
    runCheck: ({
      repoPath,
      ref,
      argotDir,
      staged,
      unstaged,
      commit,
      only,
      exclude,
      verbose,
      minSeverity,
      threshold,
    }: {
      repoPath: string;
      ref: string;
      argotDir: string;
      staged: boolean;
      unstaged: boolean;
      commit: string | undefined;
      only: ReadonlyArray<string>;
      exclude: ReadonlyArray<string>;
      verbose: boolean;
      minSeverity: MinSeverity;
      threshold: number | undefined;
    }) =>
      Effect.callback<boolean, CheckExitNonZero | CheckSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.check');

        // Build positional args: always pass repoPath; only pass ref if non-empty
        const positionals = ref.length > 0 ? [repoPath, ref] : [repoPath];

        const flags: string[] = ['--argot-dir', argotDir];
        if (staged) flags.push('--staged');
        if (unstaged) flags.push('--unstaged');
        if (commit !== undefined) flags.push('--commit', commit);
        for (const glob of only) flags.push('--only', glob);
        for (const glob of exclude) flags.push('--exclude', glob);
        if (verbose) flags.push('--verbose');
        // Default 'unusual' = no filter; only forward when the user changed it.
        if (minSeverity !== 'unusual') flags.push('--min-severity', minSeverity);
        if (threshold !== undefined) flags.push('--threshold', String(threshold));

        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(cmd, [...args, ...positionals, ...flags], {
            stdio: ['ignore', 'inherit', 'pipe'],
          });
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
