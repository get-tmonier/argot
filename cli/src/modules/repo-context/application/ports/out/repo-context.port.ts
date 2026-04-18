import { Context } from 'effect';
import type { Effect } from 'effect';
import type { ResolvedContext, RepoStatus } from '#modules/repo-context/domain/repo-context.ts';
import type { RepoContextError } from '#modules/repo-context/domain/errors.ts';
import type { GlobalSettings } from '#modules/repo-context/domain/settings.ts';

interface RepoContextShape {
  readonly resolveContext: () => Effect.Effect<ResolvedContext, RepoContextError>;
  readonly listRepos: () => Effect.Effect<RepoStatus[], RepoContextError>;
  readonly readGlobalSettings: () => Effect.Effect<GlobalSettings, RepoContextError>;
}

export class RepoContext extends Context.Service<RepoContext, RepoContextShape>()(
  '@argot/RepoContext',
) {}
