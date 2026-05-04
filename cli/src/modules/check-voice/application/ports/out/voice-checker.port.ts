import { Context } from 'effect';
import type { Effect } from 'effect';
import type { CheckError } from '#modules/check-voice/domain/errors.ts';

export type MinSeverity = 'unusual' | 'suspicious' | 'foreign';

interface VoiceCheckerShape {
  readonly runCheck: (args: {
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
  }) => Effect.Effect<boolean, CheckError>;
}

export class VoiceChecker extends Context.Service<VoiceChecker, VoiceCheckerShape>()(
  '@argot/VoiceChecker',
) {}
