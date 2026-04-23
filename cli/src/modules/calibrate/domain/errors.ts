import { Data } from 'effect';

export class CalibrateExitNonZero extends Data.TaggedError('CalibrateExitNonZero')<{
  readonly stderr: string;
  readonly exitCode: number;
}> {
  get message() {
    return `argot-calibrate exited with code ${this.exitCode}: ${this.stderr}`;
  }
}

export class CalibrateSpawnFailed extends Data.TaggedError('CalibrateSpawnFailed')<{
  readonly cause: unknown;
}> {
  get message() {
    return `Failed to spawn argot-calibrate process`;
  }
}

export type CalibrateError = CalibrateExitNonZero | CalibrateSpawnFailed;
