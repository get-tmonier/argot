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

export type RepoContextError = GitRootNotFound | SettingsReadError;
