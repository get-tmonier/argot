import { Context } from 'effect';
import type { Effect } from 'effect';
import type { CheckError } from '#modules/check-voice/domain/errors.ts';

interface VoiceCheckerShape {
  readonly runCheck: (args: {
    repoPath: string;
    ref: string;
    argotDir: string;
    threshold: number;
  }) => Effect.Effect<boolean, CheckError>;
}

export class VoiceChecker extends Context.Service<VoiceChecker, VoiceCheckerShape>()(
  '@argot/VoiceChecker',
) {}
