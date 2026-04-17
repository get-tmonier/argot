import { ServiceMap } from 'effect';
import type { Effect } from 'effect';
import type { CheckError } from '#modules/check-style/domain/errors.ts';

interface StyleCheckerShape {
  readonly runCheck: (args: {
    repoPath: string;
    ref: string;
    modelPath: string;
  }) => Effect.Effect<void, CheckError>;
}

export class StyleChecker extends ServiceMap.Service<StyleChecker, StyleCheckerShape>()(
  '@argot/StyleChecker',
) {}
