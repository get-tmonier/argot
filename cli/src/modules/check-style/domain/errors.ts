import { Data } from 'effect';

export class CheckExitNonZero extends Data.TaggedError('CheckExitNonZero')<{
  readonly stderr: string;
  readonly exitCode: number;
}> {
  get message() {
    return `Check process exited with code ${this.exitCode}: ${this.stderr}`;
  }
}

export class CheckSpawnFailed extends Data.TaggedError('CheckSpawnFailed')<{
  readonly cause: unknown;
}> {
  get message() {
    return `Failed to spawn check process`;
  }
}

export type CheckError = CheckExitNonZero | CheckSpawnFailed;
