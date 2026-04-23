import { Context } from 'effect';
import type { Effect } from 'effect';
import type { CheckError } from '#modules/check-style/domain/errors.ts';

interface StyleCheckerShape {
  readonly runCheck: (args: {
    repoPath: string;
    ref: string;
    argotDir: string;
    threshold: number;
  }) => Effect.Effect<boolean, CheckError>;
}

export class StyleChecker extends Context.Service<StyleChecker, StyleCheckerShape>()(
  '@argot/StyleChecker',
) {}
