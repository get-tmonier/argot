import { Data } from 'effect';

export class GitRootNotFound extends Data.TaggedError('GitRootNotFound')<{}> {
  get message() {
    return 'Not inside a git repository. Run argot from within a git repo.';
  }
}

export class SettingsReadError extends Data.TaggedError('SettingsReadError')<{
  readonly cause: unknown;
}> {
  get message() {
    return 'Failed to read argot settings';
  }
}

export class ScopeConfigInvalid extends Data.TaggedError('ScopeConfigInvalid')<{
  readonly cause: unknown;
}> {}

export class ScopeNotFound extends Data.TaggedError('ScopeNotFound')<{
  readonly name: string;
  readonly available: readonly string[];
}> {}

export type RepoContextError =
  | GitRootNotFound
  | SettingsReadError
  | ScopeConfigInvalid
  | ScopeNotFound;
