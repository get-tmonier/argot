import { Data } from 'effect';

export class ExplainEngineSpawnFailed extends Data.TaggedError('ExplainEngineSpawnFailed')<{
  cause: unknown;
}> {
  get message() {
    return `Failed to spawn explain engine`;
  }
}

export class ExplainEngineExitNonZero extends Data.TaggedError('ExplainEngineExitNonZero')<{
  exitCode: number;
  stderr: string;
}> {
  get message() {
    return `Explain engine exited with code ${this.exitCode}: ${this.stderr}`;
  }
}

export class ClaudeSpawnFailed extends Data.TaggedError('ClaudeSpawnFailed')<{
  cause: unknown;
}> {
  get message() {
    return `Failed to spawn claude`;
  }
}

export class ClaudeExitNonZero extends Data.TaggedError('ClaudeExitNonZero')<{
  exitCode: number;
  stderr: string;
}> {
  get message() {
    return `claude exited with code ${this.exitCode}: ${this.stderr}`;
  }
}

export class ClaudeResponseInvalid extends Data.TaggedError('ClaudeResponseInvalid')<{
  raw: string;
  cause: unknown;
}> {
  get message() {
    return `Claude returned an unexpected response format`;
  }
}

export type ExplainError =
  | ExplainEngineSpawnFailed
  | ExplainEngineExitNonZero
  | ClaudeSpawnFailed
  | ClaudeExitNonZero
  | ClaudeResponseInvalid;
