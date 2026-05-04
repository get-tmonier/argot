import { Effect } from 'effect';
import {
  VoiceChecker,
  type MinSeverity,
} from '#modules/check-voice/application/ports/out/voice-checker.port.ts';
import type { CheckError } from '#modules/check-voice/domain/errors.ts';

export const runCheckVoice = (args: {
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
}): Effect.Effect<boolean, CheckError, VoiceChecker> =>
  Effect.gen(function* () {
    const voiceChecker = yield* VoiceChecker;
    return yield* voiceChecker.runCheck(args);
  });
