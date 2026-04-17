import { Data } from 'effect';

export class TrainExitNonZero extends Data.TaggedError('TrainExitNonZero')<{
  readonly stderr: string;
  readonly exitCode: number;
}> {
  get message() {
    return `Train process exited with code ${this.exitCode}: ${this.stderr}`;
  }
}

export class TrainSpawnFailed extends Data.TaggedError('TrainSpawnFailed')<{
  readonly cause: unknown;
}> {
  get message() {
    return `Failed to spawn train process`;
  }
}

export type TrainError = TrainExitNonZero | TrainSpawnFailed;
