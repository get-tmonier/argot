import { Data } from 'effect';

export class RepoNotFound extends Data.TaggedError('RepoNotFound')<{
  readonly path: string;
}> {
  get message() {
    return `Repository not found at path: ${this.path}`;
  }
}

export class EngineExitNonZero extends Data.TaggedError('EngineExitNonZero')<{
  readonly stderr: string;
  readonly exitCode: number;
}> {
  get message() {
    return `Engine exited with code ${this.exitCode}: ${this.stderr}`;
  }
}

export class EngineSpawnFailed extends Data.TaggedError('EngineSpawnFailed')<{
  readonly cause: unknown;
}> {
  get message() {
    return `Failed to spawn engine process`;
  }
}

export type EngineError = RepoNotFound | EngineExitNonZero | EngineSpawnFailed;
