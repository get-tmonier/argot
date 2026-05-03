import { Effect } from 'effect';
import { VoiceChecker } from '#modules/check-voice/application/ports/out/voice-checker.port.ts';
import type { CheckError } from '#modules/check-voice/domain/errors.ts';

export const runCheckVoice = (args: {
  repoPath: string;
  ref: string;
  argotDir: string;
  threshold: number;
}): Effect.Effect<boolean, CheckError, VoiceChecker> =>
  Effect.gen(function* () {
    const voiceChecker = yield* VoiceChecker;
    return yield* voiceChecker.runCheck(args);
  });
