# Multi-Repo UX & CLI Verbosity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add per-repo identity, a global repo registry, `argot status`/`argot list` commands, and uv install progress — removing all manual `--repo`/`--model`/`--dataset` flags so argot auto-resolves context from the git root.

**Architecture:** New `repo-context` module (domain → port → `FsRepoContextLive` adapter) provides `RepoContext` service used by all commands. Global settings at `~/.argot/settings.json` track registered repos; local `.argot/settings.json` overrides preferences per-repo. A shared `spawn-with-progress` utility intercepts uvx stderr to show a spinner during package install.

**Tech Stack:** TypeScript/Bun, Effect 4.x (`Effect.callback`, `Effect.tryPromise`, `Context.Service`, `Layer.effect`), Node.js `child_process`, `fs/promises`, `node:path`, `node:os`.

**Spec:** `docs/superpowers/specs/2026-04-18-multi-repo-ux-design.md`

---

## File Map

### New files
| Path | Purpose |
|---|---|
| `cli/src/modules/repo-context/domain/settings.ts` | Types for global/local settings + `mergePreferences` |
| `cli/src/modules/repo-context/domain/repo-context.ts` | `ResolvedContext`, `RepoStatus`, `DatasetInfo`, `ModelInfo` |
| `cli/src/modules/repo-context/domain/errors.ts` | `GitRootNotFound`, `SettingsReadError`, `RepoContextError` |
| `cli/src/modules/repo-context/application/ports/out/repo-context.port.ts` | `RepoContext` Effect service tag |
| `cli/src/modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts` | Full fs-backed implementation |
| `cli/src/modules/repo-context/dependencies.ts` | `RepoContextLive` export |
| `cli/src/spawn-with-progress.ts` | `handleUvStderr` utility for uv spinner |
| `cli/src/shell/infrastructure/adapters/in/commands/status.command.ts` | `argot status` |
| `cli/src/shell/infrastructure/adapters/in/commands/list.command.ts` | `argot list` |

### Modified files
| Path | Change |
|---|---|
| `cli/src/modules/check-style/application/ports/out/style-checker.port.ts` | Add `threshold` param |
| `cli/src/modules/check-style/application/use-cases/check-style.use-case.ts` | Pass `threshold` |
| `cli/src/modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts` | Pass `--threshold`, add uv spinner |
| `cli/src/modules/explain/application/ports/out/explainer.port.ts` | Add `claudeModel` param |
| `cli/src/modules/explain/application/use-cases/explain.use-case.ts` | Pass `claudeModel` |
| `cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts` | Pass `--model` to claude CLI, add uv spinner |
| `cli/src/modules/extract-dataset/infrastructure/adapters/out/bun-engine-runner.adapter.ts` | Add uv spinner |
| `cli/src/modules/train-model/infrastructure/adapters/out/bun-model-trainer.adapter.ts` | Add uv spinner |
| `cli/src/shell/infrastructure/adapters/in/commands/extract.command.ts` | Remove flags, use `RepoContext` |
| `cli/src/shell/infrastructure/adapters/in/commands/train.command.ts` | Remove flags, use `RepoContext` |
| `cli/src/shell/infrastructure/adapters/in/commands/check.command.ts` | Remove flags, use `RepoContext` |
| `cli/src/shell/infrastructure/adapters/in/commands/explain.command.ts` | Remove flags, use `RepoContext` |
| `cli/src/dependencies.ts` | Add `RepoContextLive` |
| `cli/src/cli.ts` | Register `status`, `list`; update help text |

### Test files
| Path | Purpose |
|---|---|
| `cli/src/modules/repo-context/domain/settings.test.ts` | Unit tests for `mergePreferences` |

---

## Task 1: Settings domain types + merge function

**Files:**
- Create: `cli/src/modules/repo-context/domain/settings.ts`
- Create (test): `cli/src/modules/repo-context/domain/settings.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
// cli/src/modules/repo-context/domain/settings.test.ts
import { describe, expect, it } from 'bun:test';
import { mergePreferences, DEFAULT_PREFERENCES } from './settings.ts';

describe('mergePreferences', () => {
  it('returns global when no local override', () => {
    expect(mergePreferences(DEFAULT_PREFERENCES, undefined)).toEqual(DEFAULT_PREFERENCES);
  });

  it('overrides threshold from local settings', () => {
    const result = mergePreferences(DEFAULT_PREFERENCES, { threshold: 0.3 });
    expect(result.threshold).toBe(0.3);
    expect(result.model).toBe('sonnet');
  });

  it('overrides model from local settings', () => {
    const result = mergePreferences(DEFAULT_PREFERENCES, { model: 'opus' });
    expect(result.model).toBe('opus');
    expect(result.threshold).toBe(0.5);
  });

  it('overrides both fields simultaneously', () => {
    const result = mergePreferences(DEFAULT_PREFERENCES, { threshold: 0.2, model: 'haiku' });
    expect(result).toEqual({ threshold: 0.2, model: 'haiku' });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd cli && bun test src/modules/repo-context/domain/settings.test.ts
```
Expected: error — module not found.

- [ ] **Step 3: Write the implementation**

```typescript
// cli/src/modules/repo-context/domain/settings.ts

export interface Preferences {
  threshold: number;
  model: string;
}

export const DEFAULT_PREFERENCES: Preferences = {
  threshold: 0.5,
  model: 'sonnet',
};

export interface RepoEntry {
  name: string;
  registeredAt: string;
  lastUsedAt: string;
}

export interface GlobalSettings {
  version: number;
  preferences: Preferences;
  repos: Record<string, RepoEntry>;
}

export const DEFAULT_GLOBAL_SETTINGS: GlobalSettings = {
  version: 1,
  preferences: DEFAULT_PREFERENCES,
  repos: {},
};

export interface LocalSettings {
  preferences?: Partial<Preferences>;
}

export function mergePreferences(
  global: Preferences,
  local: Partial<Preferences> | undefined,
): Preferences {
  if (!local) return global;
  return { ...global, ...local };
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd cli && bun test src/modules/repo-context/domain/settings.test.ts
```
Expected: 4 passing.

- [ ] **Step 5: Commit**

```bash
git add cli/src/modules/repo-context/domain/settings.ts cli/src/modules/repo-context/domain/settings.test.ts
git commit -m "feat(repo-context): settings domain types and merge function"
```

---

## Task 2: RepoContext domain types

**Files:**
- Create: `cli/src/modules/repo-context/domain/repo-context.ts`

- [ ] **Step 1: Write the file**

```typescript
// cli/src/modules/repo-context/domain/repo-context.ts

import type { Preferences } from './settings.ts';

export interface ResolvedContext {
  gitRoot: string;
  name: string;
  datasetPath: string;  // <gitRoot>/.argot/dataset.jsonl
  modelPath: string;    // <gitRoot>/.argot/model.pkl
  preferences: Preferences;
}

export interface DatasetInfo {
  sizeBytes: number;
  mtime: Date;
}

export interface ModelInfo {
  sizeBytes: number;
  mtime: Date;
}

export interface RepoStatus {
  path: string;
  name: string;
  isCurrent: boolean;
  datasetInfo: DatasetInfo | null;
  modelInfo: ModelInfo | null;
}
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/modules/repo-context/domain/repo-context.ts
git commit -m "feat(repo-context): domain types for resolved context and repo status"
```

---

## Task 3: RepoContext errors

**Files:**
- Create: `cli/src/modules/repo-context/domain/errors.ts`

- [ ] **Step 1: Write the file**

```typescript
// cli/src/modules/repo-context/domain/errors.ts

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
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/modules/repo-context/domain/errors.ts
git commit -m "feat(repo-context): error types"
```

---

## Task 4: RepoContext port (service interface)

**Files:**
- Create: `cli/src/modules/repo-context/application/ports/out/repo-context.port.ts`

- [ ] **Step 1: Write the file**

```typescript
// cli/src/modules/repo-context/application/ports/out/repo-context.port.ts

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
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/modules/repo-context/application/ports/out/repo-context.port.ts
git commit -m "feat(repo-context): RepoContext service port"
```

---

## Task 5: FsRepoContext adapter

**Files:**
- Create: `cli/src/modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts`

- [ ] **Step 1: Write the file**

```typescript
// cli/src/modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts

import { spawn } from 'node:child_process';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { Effect, Layer } from 'effect';
import { RepoContext } from '#modules/repo-context/application/ports/out/repo-context.port.ts';
import { GitRootNotFound, SettingsReadError } from '#modules/repo-context/domain/errors.ts';
import type { DatasetInfo, ModelInfo, RepoStatus } from '#modules/repo-context/domain/repo-context.ts';
import {
  DEFAULT_GLOBAL_SETTINGS,
  mergePreferences,
  type GlobalSettings,
  type LocalSettings,
} from '#modules/repo-context/domain/settings.ts';

const GLOBAL_SETTINGS_PATH = join(homedir(), '.argot', 'settings.json');

async function getGitRootAsync(): Promise<string | null> {
  return new Promise((resolve) => {
    const proc = spawn('git', ['rev-parse', '--show-toplevel'], {
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    const chunks: Buffer[] = [];
    proc.stdout!.on('data', (chunk: Buffer) => chunks.push(chunk));
    proc.on('close', (code) => {
      resolve(code === 0 ? Buffer.concat(chunks).toString('utf-8').trim() : null);
    });
    proc.on('error', () => resolve(null));
  });
}

async function readGlobalSettingsAsync(): Promise<GlobalSettings> {
  try {
    const content = await readFile(GLOBAL_SETTINGS_PATH, 'utf-8');
    return JSON.parse(content) as GlobalSettings;
  } catch {
    return structuredClone(DEFAULT_GLOBAL_SETTINGS);
  }
}

async function writeGlobalSettingsAsync(settings: GlobalSettings): Promise<void> {
  await mkdir(dirname(GLOBAL_SETTINGS_PATH), { recursive: true });
  await writeFile(GLOBAL_SETTINGS_PATH, JSON.stringify(settings, null, 2));
}

async function readLocalSettingsAsync(gitRoot: string): Promise<LocalSettings | null> {
  try {
    const content = await readFile(join(gitRoot, '.argot', 'settings.json'), 'utf-8');
    return JSON.parse(content) as LocalSettings;
  } catch {
    return null;
  }
}

async function statOrNull(path: string): Promise<{ sizeBytes: number; mtime: Date } | null> {
  try {
    const s = await stat(path);
    return { sizeBytes: s.size, mtime: s.mtime };
  } catch {
    return null;
  }
}

export const FsRepoContextLive = Layer.effect(RepoContext)(
  Effect.succeed({
    resolveContext: () =>
      Effect.tryPromise({
        try: async () => {
          const gitRoot = (await getGitRootAsync()) ?? process.cwd();
          const global = await readGlobalSettingsAsync();
          const local = await readLocalSettingsAsync(gitRoot);
          const preferences = mergePreferences(global.preferences, local?.preferences);

          const now = new Date().toISOString();
          if (!global.repos[gitRoot]) {
            global.repos[gitRoot] = {
              name: basename(gitRoot),
              registeredAt: now,
              lastUsedAt: now,
            };
          } else {
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            global.repos[gitRoot]!.lastUsedAt = now;
          }
          await writeGlobalSettingsAsync(global);

          return {
            gitRoot,
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            name: global.repos[gitRoot]!.name,
            datasetPath: join(gitRoot, '.argot', 'dataset.jsonl'),
            modelPath: join(gitRoot, '.argot', 'model.pkl'),
            preferences,
          };
        },
        catch: (e) => new SettingsReadError({ cause: e }),
      }),

    listRepos: () =>
      Effect.tryPromise({
        try: async () => {
          const currentRoot = await getGitRootAsync();
          const global = await readGlobalSettingsAsync();

          const results: RepoStatus[] = [];
          for (const [path, entry] of Object.entries(global.repos)) {
            const datasetInfo: DatasetInfo | null = await statOrNull(
              join(path, '.argot', 'dataset.jsonl'),
            );
            const modelInfo: ModelInfo | null = await statOrNull(join(path, '.argot', 'model.pkl'));
            results.push({
              path,
              name: entry.name,
              isCurrent: path === currentRoot,
              datasetInfo,
              modelInfo,
            });
          }
          return results.sort((a, b) => a.name.localeCompare(b.name));
        },
        catch: (e) => new SettingsReadError({ cause: e }),
      }),

    readGlobalSettings: () =>
      Effect.tryPromise({
        try: () => readGlobalSettingsAsync(),
        catch: (e) => new SettingsReadError({ cause: e }),
      }),
  }),
);
```

Note: `gitRoot` from `getGitRootAsync()` is always defined as a key in `global.repos` after the registration block, so the non-null assertion on `global.repos[gitRoot]!.name` is safe. The `#spawn-with-progress.ts` alias is added to `cli/package.json` imports in Task 8 — wait until Task 8 before adding that alias.

- [ ] **Step 2: Verify TypeScript accepts it**

```bash
cd cli && bunx tsgo --noEmit 2>&1 | grep repo-context
```
Expected: no errors for repo-context files.

- [ ] **Step 3: Commit**

```bash
git add cli/src/modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts
git commit -m "feat(repo-context): FsRepoContext adapter with global settings and auto-registration"
```

---

## Task 6: repo-context module dependencies.ts

**Files:**
- Create: `cli/src/modules/repo-context/dependencies.ts`

- [ ] **Step 1: Write the file**

```typescript
// cli/src/modules/repo-context/dependencies.ts

import { FsRepoContextLive } from '#modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts';

export { RepoContext } from '#modules/repo-context/application/ports/out/repo-context.port.ts';
export const RepoContextLive = FsRepoContextLive;
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/modules/repo-context/dependencies.ts
git commit -m "feat(repo-context): module dependencies export"
```

---

## Task 7: spawn-with-progress utility (uv spinner)

**Files:**
- Create: `cli/src/spawn-with-progress.ts`

- [ ] **Step 1: Write the file**

```typescript
// cli/src/spawn-with-progress.ts

const UV_LINE_PATTERN = /^(Resolved|Prepared|Installed|Downloading|Audited|Updated)\b/;
const SPINNER_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

function createUvSpinner(message: string): { stop: () => void } {
  let frame = 0;
  const interval = setInterval(() => {
    process.stderr.write(
      `\r${SPINNER_FRAMES[frame++ % SPINNER_FRAMES.length]} ${message}  `,
    );
  }, 80);
  return {
    stop() {
      clearInterval(interval);
      process.stderr.write('\r\x1b[K');
    },
  };
}

/**
 * Pipes stderr from a subprocess with a uv install spinner.
 * Returns a cleanup function — call it in the process 'close' handler before resuming.
 * Non-uv stderr lines are forwarded to onErrorChunk for error accumulation.
 */
export function handleUvStderr(
  stderrStream: NodeJS.ReadableStream,
  onErrorChunk: (chunk: Buffer) => void,
): () => void {
  let spinner: ReturnType<typeof createUvSpinner> | null = null;

  stderrStream.on('data', (chunk: Buffer) => {
    const text = chunk.toString('utf-8');
    const nonUvLines: string[] = [];

    for (const line of text.split('\n')) {
      if (UV_LINE_PATTERN.test(line.trim())) {
        if (!spinner) spinner = createUvSpinner('Installing argot-engine…');
      } else if (line.trim()) {
        nonUvLines.push(line);
      }
    }

    if (nonUvLines.length > 0) {
      onErrorChunk(Buffer.from(nonUvLines.join('\n')));
    }
  });

  return () => {
    spinner?.stop();
    spinner = null;
  };
}
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/spawn-with-progress.ts
git commit -m "feat: uv install spinner utility for engine subprocess stderr"
```

---

## Task 8: Add uv spinner to all four engine adapters

**Files:**
- Modify: `cli/src/modules/extract-dataset/infrastructure/adapters/out/bun-engine-runner.adapter.ts`
- Modify: `cli/src/modules/train-model/infrastructure/adapters/out/bun-model-trainer.adapter.ts`
- Modify: `cli/src/modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts`
- Modify: `cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts`

The pattern is the same for each: replace `proc.stderr!.on('data', ...)` with `handleUvStderr`, and call the returned stop function in the `close` handler.

- [ ] **Step 1: Update bun-engine-runner.adapter.ts**

Replace the existing file contents with:

```typescript
import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '../../../../../../../spawn-with-progress.ts';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import { EngineExitNonZero, EngineSpawnFailed } from '#modules/extract-dataset/domain/errors.ts';

export const BunEngineRunnerLive = Layer.effect(EngineRunner)(
  Effect.succeed({
    runExtract: ({ repoPath, outputPath }: { repoPath: string; outputPath: string }) =>
      Effect.callback<void, EngineExitNonZero | EngineSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.extract');
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(cmd, [...args, repoPath, '--out', outputPath], {
            stdio: ['ignore', 'inherit', 'pipe'],
          });
        } catch (cause: unknown) {
          resume(Effect.fail(new EngineSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new EngineSpawnFailed({ cause })));
        });

        proc.on('close', (code: number | null) => {
          stopSpinner();
          if (code === 0) {
            resume(Effect.void);
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new EngineExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
```

Note the import path for `spawn-with-progress.ts`: use a relative path from the adapter file's location. From `cli/src/modules/extract-dataset/infrastructure/adapters/out/`, the relative path to `cli/src/spawn-with-progress.ts` is `'../../../../../../../spawn-with-progress.ts'`. Alternatively, add `"#spawn-with-progress.ts": "./src/spawn-with-progress.ts"` to `cli/package.json` imports (recommended — do this first).

- [ ] **Step 2: Add path alias to cli/package.json**

In `cli/package.json`, in the `"imports"` object, add:
```json
"#spawn-with-progress.ts": "./src/spawn-with-progress.ts"
```

Then use `import { handleUvStderr } from '#spawn-with-progress.ts';` in all four adapters.

- [ ] **Step 3: Update bun-model-trainer.adapter.ts**

Read the current file and apply the same pattern — replace the stderr collection with `handleUvStderr` and call `stopSpinner()` in the close handler.

```typescript
import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { ModelTrainer } from '#modules/train-model/application/ports/out/model-trainer.port.ts';
import { TrainExitNonZero, TrainSpawnFailed } from '#modules/train-model/domain/errors.ts';

export const BunModelTrainerLive = Layer.effect(ModelTrainer)(
  Effect.succeed({
    runTrain: ({ datasetPath, modelPath }: { datasetPath: string; modelPath: string }) =>
      Effect.callback<void, TrainExitNonZero | TrainSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.train');
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(cmd, [...args, '--dataset', datasetPath, '--out', modelPath], {
            stdio: ['ignore', 'inherit', 'pipe'],
          });
        } catch (cause: unknown) {
          resume(Effect.fail(new TrainSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new TrainSpawnFailed({ cause })));
        });

        proc.on('close', (code: number | null) => {
          stopSpinner();
          if (code === 0) {
            resume(Effect.void);
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new TrainExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
```

- [ ] **Step 4: Update bun-explainer.adapter.ts (spinner only — claudeModel added in Task 10)**

The explain adapter has two subprocess spawns (engine + claude). Only the engine spawn needs the uv spinner. In `bun-explainer.adapter.ts`, add the import and replace the two stderr lines with `handleUvStderr`, then call `stopSpinner()` in the close handler:

Add at top:
```typescript
import { handleUvStderr } from '#spawn-with-progress.ts';
```

Replace:
```typescript
const stderrChunks: Buffer[] = [];
proc.stderr!.on('data', (chunk: Buffer) => stderrChunks.push(chunk));
```

With:
```typescript
const stderrChunks: Buffer[] = [];
const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));
```

And in the `proc.on('close', ...)` handler, add `stopSpinner();` as the first line inside the `Promise.all(lines).then(() => {` callback:
```typescript
proc.on('close', (code: number | null) => {
  Promise.all(lines).then(() => {
    stopSpinner();
    if (code === 0) {
      resume(Effect.void);
    } else {
      const stderr = Buffer.concat(stderrChunks).toString('utf-8');
      resume(Effect.fail(new ExplainEngineExitNonZero({ exitCode: code ?? -1, stderr })));
    }
  });
});
```

Note: `bun-style-checker.adapter.ts` is NOT updated here — Task 9 rewrites it completely including the spinner.

- [ ] **Step 6: Verify TypeScript**

```bash
cd cli && bunx tsgo --noEmit 2>&1 | grep -E "error"
```
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add cli/package.json \
  cli/src/modules/extract-dataset/infrastructure/adapters/out/bun-engine-runner.adapter.ts \
  cli/src/modules/train-model/infrastructure/adapters/out/bun-model-trainer.adapter.ts \
  cli/src/modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts \
  cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts
git commit -m "feat: uv install spinner in all engine subprocess adapters"
```

---

## Task 9: Add threshold to check-style port, use-case, and adapter

**Files:**
- Modify: `cli/src/modules/check-style/application/ports/out/style-checker.port.ts`
- Modify: `cli/src/modules/check-style/application/use-cases/check-style.use-case.ts`
- Modify: `cli/src/modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts`

- [ ] **Step 1: Update style-checker.port.ts** — add `threshold: number` to `runCheck` args

```typescript
import { Context } from 'effect';
import type { Effect } from 'effect';
import type { CheckError } from '#modules/check-style/domain/errors.ts';

interface StyleCheckerShape {
  readonly runCheck: (args: {
    repoPath: string;
    ref: string;
    modelPath: string;
    threshold: number;
  }) => Effect.Effect<void, CheckError>;
}

export class StyleChecker extends Context.Service<StyleChecker, StyleCheckerShape>()(
  '@argot/StyleChecker',
) {}
```

- [ ] **Step 2: Update check-style.use-case.ts** — add `threshold` param and pass it through

```typescript
import { Effect } from 'effect';
import { StyleChecker } from '#modules/check-style/application/ports/out/style-checker.port.ts';
import type { CheckError } from '#modules/check-style/domain/errors.ts';

export const runCheckStyle = (args: {
  repoPath: string;
  ref: string;
  modelPath: string;
  threshold: number;
}): Effect.Effect<void, CheckError, StyleChecker> =>
  Effect.gen(function* () {
    const styleChecker = yield* StyleChecker;
    yield* styleChecker.runCheck(args);
  });
```

- [ ] **Step 3: Update bun-style-checker.adapter.ts** — add `threshold` to method signature and pass `--threshold` to Python

```typescript
import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { StyleChecker } from '#modules/check-style/application/ports/out/style-checker.port.ts';
import { CheckExitNonZero, CheckSpawnFailed } from '#modules/check-style/domain/errors.ts';

export const BunStyleCheckerLive = Layer.effect(StyleChecker)(
  Effect.succeed({
    runCheck: ({
      repoPath,
      ref,
      modelPath,
      threshold,
    }: {
      repoPath: string;
      ref: string;
      modelPath: string;
      threshold: number;
    }) =>
      Effect.callback<void, CheckExitNonZero | CheckSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.check');
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(
            cmd,
            [...args, repoPath, ref, '--model', modelPath, '--threshold', String(threshold)],
            { stdio: ['ignore', 'inherit', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new CheckSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) => {
          resume(Effect.fail(new CheckSpawnFailed({ cause })));
        });
        proc.on('close', (code: number | null) => {
          stopSpinner();
          if (code === 0) {
            resume(Effect.void);
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new CheckExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
```

- [ ] **Step 4: Verify TypeScript**

```bash
cd cli && bunx tsgo --noEmit 2>&1 | grep -E "error"
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add cli/src/modules/check-style/application/ports/out/style-checker.port.ts \
  cli/src/modules/check-style/application/use-cases/check-style.use-case.ts \
  cli/src/modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts
git commit -m "feat(check-style): pass threshold from settings to Python engine"
```

---

## Task 10: Add claudeModel to explain port, use-case, and adapter

**Files:**
- Modify: `cli/src/modules/explain/application/ports/out/explainer.port.ts`
- Modify: `cli/src/modules/explain/application/use-cases/explain.use-case.ts`
- Modify: `cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts`

- [ ] **Step 1: Update explainer.port.ts** — add `claudeModel: string`

```typescript
// cli/src/modules/explain/application/ports/out/explainer.port.ts
import { Context } from 'effect';
import type { Effect } from 'effect';
import type { ExplainError } from '#modules/explain/domain/errors.ts';

interface ExplainerShape {
  readonly runExplain: (args: {
    repoPath: string;
    ref: string;
    modelPath: string;
    datasetPath: string;
    claudeModel: string;
  }) => Effect.Effect<void, ExplainError>;
}

export class Explainer extends Context.Service<Explainer, ExplainerShape>()('@argot/Explainer') {}
```

- [ ] **Step 2: Update explain.use-case.ts** — pass `claudeModel` through

```typescript
// cli/src/modules/explain/application/use-cases/explain.use-case.ts
import { Effect } from 'effect';
import { Explainer } from '#modules/explain/application/ports/out/explainer.port.ts';
import type { ExplainError } from '#modules/explain/domain/errors.ts';

export const runExplain = (args: {
  repoPath: string;
  ref: string;
  modelPath: string;
  datasetPath: string;
  claudeModel: string;
}): Effect.Effect<void, ExplainError, Explainer> =>
  Effect.gen(function* () {
    const explainer = yield* Explainer;
    yield* explainer.runExplain(args);
  });
```

- [ ] **Step 3: Update bun-explainer.adapter.ts** — accept `claudeModel` and pass `--model` to claude CLI

The `callClaude` function needs `claudeModel` as a parameter. Thread it from `runExplain` args. Replace the entire file:

```typescript
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { Console, Effect, Layer, Schema } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { Explainer } from '#modules/explain/application/ports/out/explainer.port.ts';
import {
  ClaudeExitNonZero,
  ClaudeResponseInvalid,
  ClaudeSpawnFailed,
  ExplainEngineExitNonZero,
  ExplainEngineSpawnFailed,
} from '#modules/explain/domain/errors.ts';

const EngineRecord = Schema.Struct({
  file_path: Schema.String,
  line: Schema.Number,
  commit: Schema.String,
  surprise: Schema.Number,
  percentile: Schema.Number,
  hunk_text: Schema.String,
  style_examples: Schema.Array(Schema.String),
});

const Explanation = Schema.Struct({
  summary: Schema.String,
  issues: Schema.Array(Schema.String),
});

const ClaudeEnvelope = Schema.Struct({
  structured_output: Schema.NullOr(Explanation),
  result: Schema.String,
});

const CLAUDE_SCHEMA = JSON.stringify({
  type: 'object',
  properties: {
    summary: { type: 'string' },
    issues: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'issues'],
});

const callClaude = (
  record: typeof EngineRecord.Type,
  claudeModel: string,
): Effect.Effect<
  typeof Explanation.Type,
  ClaudeSpawnFailed | ClaudeExitNonZero | ClaudeResponseInvalid
> =>
  Effect.gen(function* () {
    const examples = record.style_examples.map((ex, i) => `[${i + 1}] ${ex}`).join('\n---\n');

    const prompt =
      `This codebase's typical style (lowest-surprise commits from training data):\n\n` +
      `<examples>\n${examples}\n</examples>\n\n` +
      `This hunk from ${record.file_path}:${record.line} scored p${record.percentile} surprise ` +
      `(commit ${record.commit}):\n\n` +
      `<hunk>\n${record.hunk_text}\n</hunk>\n\n` +
      `In 2-3 sentences, what specific style differences do you see? Be concrete about naming, ` +
      `structure, patterns, line length. Return JSON only.`;

    const raw = yield* Effect.callback<string, ClaudeSpawnFailed | ClaudeExitNonZero>((resume) => {
      let proc: ReturnType<typeof spawn>;
      try {
        proc = spawn(
          'claude',
          [
            '--print',
            '--output-format', 'json',
            '--model', claudeModel,
            '--tools', '',
            '--json-schema', CLAUDE_SCHEMA,
            prompt,
          ],
          { stdio: ['ignore', 'pipe', 'pipe'] },
        );
      } catch (cause: unknown) {
        resume(Effect.fail(new ClaudeSpawnFailed({ cause })));
        return;
      }

      const chunks: Buffer[] = [];
      const stderrChunks: Buffer[] = [];
      proc.stdout!.on('data', (chunk: Buffer) => chunks.push(chunk));
      proc.stderr!.on('data', (chunk: Buffer) => stderrChunks.push(chunk));
      proc.on('error', (cause: unknown) => resume(Effect.fail(new ClaudeSpawnFailed({ cause }))));
      proc.on('close', (code: number | null) => {
        if (code === 0) {
          resume(Effect.succeed(Buffer.concat(chunks).toString('utf-8')));
        } else {
          const stderr = Buffer.concat(stderrChunks).toString('utf-8');
          resume(Effect.fail(new ClaudeExitNonZero({ exitCode: code ?? -1, stderr })));
        }
      });
    });

    const envelope = yield* Schema.decodeUnknownEffect(Schema.fromJsonString(ClaudeEnvelope))(
      raw,
    ).pipe(Effect.mapError((cause) => new ClaudeResponseInvalid({ raw, cause })));
    const explanation =
      envelope.structured_output ??
      (yield* Schema.decodeUnknownEffect(Schema.fromJsonString(Explanation))(envelope.result).pipe(
        Effect.mapError((cause) => new ClaudeResponseInvalid({ raw: envelope.result, cause })),
      ));
    return explanation;
  });

export const BunExplainerLive = Layer.effect(Explainer)(
  Effect.succeed({
    runExplain: ({
      repoPath,
      ref,
      modelPath,
      datasetPath,
      claudeModel,
    }: {
      repoPath: string;
      ref: string;
      modelPath: string;
      datasetPath: string;
      claudeModel: string;
    }) =>
      Effect.callback<void, ExplainEngineSpawnFailed | ExplainEngineExitNonZero>((resume) => {
        const { cmd, args } = engineCmd('argot.explain');
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(
            cmd,
            [...args, repoPath, ref, '--model', modelPath, '--dataset', datasetPath],
            { stdio: ['ignore', 'pipe', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new ExplainEngineSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) =>
          resume(Effect.fail(new ExplainEngineSpawnFailed({ cause }))),
        );

        const rl = createInterface({ input: proc.stdout! });

        const lines: Promise<void>[] = [];
        rl.on('line', (line: string) => {
          if (!line.trim()) return;
          lines.push(
            Effect.runPromise(
              Effect.gen(function* () {
                const record = yield* Schema.decodeUnknownEffect(
                  Schema.fromJsonString(EngineRecord),
                )(line);
                const explanation = yield* callClaude(record, claudeModel);
                yield* Console.log(
                  `\n${record.file_path}:${record.line} (p${record.percentile}, commit ${record.commit})`,
                );
                yield* Console.log(`  ${explanation.summary}`);
                for (const issue of explanation.issues) {
                  yield* Console.log(`  • ${issue}`);
                }
              }),
            ),
          );
        });

        proc.on('close', (code: number | null) => {
          Promise.all(lines).then(() => {
            stopSpinner();
            if (code === 0) {
              resume(Effect.void);
            } else {
              const stderr = Buffer.concat(stderrChunks).toString('utf-8');
              resume(Effect.fail(new ExplainEngineExitNonZero({ exitCode: code ?? -1, stderr })));
            }
          });
        });
      }),
  }),
);
```

- [ ] **Step 4: Verify TypeScript**

```bash
cd cli && bunx tsgo --noEmit 2>&1 | grep -E "error"
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add cli/src/modules/explain/application/ports/out/explainer.port.ts \
  cli/src/modules/explain/application/use-cases/explain.use-case.ts \
  cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts
git commit -m "feat(explain): pass claude model from settings to Claude CLI"
```

---

## Task 11: Update extract command

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/extract.command.ts`

- [ ] **Step 1: Write the new command**

Remove the `path` argument and `out` flag. Use `RepoContext` to resolve paths and print a verbosity header.

```typescript
import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runExtractDataset } from '#modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts';

export const extractCommand = Command.make('extract', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();
    yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);
    const result = yield* runExtractDataset({
      repoPath: ctx.gitRoot,
      outputPath: ctx.datasetPath,
    });
    yield* Console.log(`Dataset written to ${result.outputPath}`);
  }),
);
```

- [ ] **Step 2: Verify TypeScript**

```bash
cd cli && bunx tsgo --noEmit 2>&1 | grep -E "error"
```
Expected: no errors (will show error about `RepoContext` not in `AppLive` — that's fixed in Task 17).

- [ ] **Step 3: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/extract.command.ts
git commit -m "feat(extract): use RepoContext for auto-resolved paths and verbosity header"
```

---

## Task 12: Update train command

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/train.command.ts`

- [ ] **Step 1: Write the new command**

Remove `--dataset` and `--model` flags:

```typescript
import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runTrainModel } from '#modules/train-model/application/use-cases/train-model.use-case.ts';

export const trainCommand = Command.make('train', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();
    yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);
    yield* runTrainModel({ datasetPath: ctx.datasetPath, modelPath: ctx.modelPath });
    yield* Console.log(`Model written to ${ctx.modelPath}`);
  }),
);
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/train.command.ts
git commit -m "feat(train): use RepoContext for auto-resolved paths and verbosity header"
```

---

## Task 13: Update check command

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/check.command.ts`

- [ ] **Step 1: Write the new command**

Remove `--model` and `--repo` flags; keep `ref` argument; read threshold from settings:

```typescript
import { Argument, Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runCheckStyle } from '#modules/check-style/application/use-cases/check-style.use-case.ts';

export const checkCommand = Command.make(
  'check',
  {
    ref: Argument.string('ref').pipe(
      Argument.withDefault(''),
      Argument.withDescription(
        'Git ref to check: bare ref (HEAD, abc1234), range (HEAD~5..HEAD), or omit to check uncommitted changes',
      ),
    ),
  },
  ({ ref }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot}) · threshold ${ctx.preferences.threshold}`);
      yield* runCheckStyle({
        repoPath: ctx.gitRoot,
        ref,
        modelPath: ctx.modelPath,
        threshold: ctx.preferences.threshold,
      });
    }),
);
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/check.command.ts
git commit -m "feat(check): use RepoContext; threshold from settings; remove --model/--repo flags"
```

---

## Task 14: Update explain command

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/explain.command.ts`

- [ ] **Step 1: Write the new command**

Remove `--model`, `--dataset`, `--repo` flags; keep `ref` argument; read `claudeModel` from settings:

```typescript
import { Argument, Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runExplain } from '#modules/explain/application/use-cases/explain.use-case.ts';

export const explainCommand = Command.make(
  'explain',
  {
    ref: Argument.string('ref').pipe(
      Argument.withDescription('Git ref to explain: bare ref (HEAD, abc1234) or range (HEAD~5..HEAD)'),
    ),
  },
  ({ ref }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot}) · model ${ctx.preferences.model}`);
      yield* runExplain({
        repoPath: ctx.gitRoot,
        ref,
        modelPath: ctx.modelPath,
        datasetPath: ctx.datasetPath,
        claudeModel: ctx.preferences.model,
      });
    }),
);
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/explain.command.ts
git commit -m "feat(explain): use RepoContext; claude model from settings; remove manual path flags"
```

---

## Task 15: argot status command

**Files:**
- Create: `cli/src/shell/infrastructure/adapters/in/commands/status.command.ts`

- [ ] **Step 1: Write the command**

```typescript
// cli/src/shell/infrastructure/adapters/in/commands/status.command.ts

import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { stat } from 'node:fs/promises';
import { join } from 'node:path';
import { RepoContext } from '#modules/repo-context/dependencies.ts';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatAge(mtime: Date): string {
  const diff = Date.now() - mtime.getTime();
  const hours = Math.floor(diff / 3_600_000);
  if (hours < 1) return 'just now';
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

async function countJsonlLines(path: string): Promise<number | null> {
  try {
    const content = await Bun.file(path).text();
    return content.split('\n').filter((l) => l.trim()).length;
  } catch {
    return null;
  }
}

export const statusCommand = Command.make('status', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();

    yield* Console.log(`Repo:     ${ctx.name} (${ctx.gitRoot})`);

    let datasetLine = '—';
    try {
      const s = await stat(ctx.datasetPath);
      const count = await countJsonlLines(ctx.datasetPath);
      const countStr = count !== null ? `${count} records · ` : '';
      datasetLine = `${countStr}${formatBytes(s.size)} · last extracted ${formatAge(s.mtime)}`;
    } catch {}
    yield* Console.log(`Dataset:  ${datasetLine}`);

    let modelLine = 'not trained';
    try {
      const s = await stat(ctx.modelPath);
      modelLine = `trained ${formatAge(s.mtime)} · ${formatBytes(s.size)}`;
    } catch {}
    yield* Console.log(`Model:    ${modelLine}`);

    const thresholdSource =
      ctx.preferences.threshold === 0.5 ? '(global default)' : '(local override)';
    yield* Console.log(
      `Settings: threshold ${ctx.preferences.threshold} ${thresholdSource} · model ${ctx.preferences.model}`,
    );
  }),
);
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/status.command.ts
git commit -m "feat: add argot status command"
```

---

## Task 16: argot list command

**Files:**
- Create: `cli/src/shell/infrastructure/adapters/in/commands/list.command.ts`

- [ ] **Step 1: Write the command**

```typescript
// cli/src/shell/infrastructure/adapters/in/commands/list.command.ts

import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { homedir } from 'node:os';
import { RepoContext } from '#modules/repo-context/dependencies.ts';

function abbreviatePath(p: string): string {
  const home = homedir();
  return p.startsWith(home) ? `~${p.slice(home.length)}` : p;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatAge(mtime: Date): string {
  const diff = Date.now() - mtime.getTime();
  const hours = Math.floor(diff / 3_600_000);
  if (hours < 1) return 'just now';
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

export const listCommand = Command.make('list', {}, () =>
  Effect.gen(function* () {
    const { listRepos } = yield* RepoContext;
    const repos = yield* listRepos();

    if (repos.length === 0) {
      yield* Console.log('No repos registered yet. Run any argot command inside a git repo.');
      return;
    }

    const NAME_W = Math.max(4, ...repos.map((r) => r.name.length));
    const PATH_W = Math.max(4, ...repos.map((r) => abbreviatePath(r.path).length));

    const header = [
      '  ',
      'NAME'.padEnd(NAME_W),
      '  ',
      'PATH'.padEnd(PATH_W),
      '  ',
      'DATASET'.padEnd(20),
      '  ',
      'MODEL',
    ].join('');
    yield* Console.log(header);

    for (const repo of repos) {
      const prefix = repo.isCurrent ? '* ' : '  ';
      const datasetCol = repo.datasetInfo
        ? `${formatBytes(repo.datasetInfo.sizeBytes)} · ${formatAge(repo.datasetInfo.mtime)}`.padEnd(20)
        : '—'.padEnd(20);
      const modelCol = repo.modelInfo ? `trained ${formatAge(repo.modelInfo.mtime)}` : 'not trained';

      const row = [
        prefix,
        repo.name.padEnd(NAME_W),
        '  ',
        abbreviatePath(repo.path).padEnd(PATH_W),
        '  ',
        datasetCol,
        '  ',
        modelCol,
      ].join('');
      yield* Console.log(row);
    }
  }),
);
```

- [ ] **Step 2: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/list.command.ts
git commit -m "feat: add argot list command showing all registered repos"
```

---

## Task 17: Wire cli.ts and root dependencies.ts

**Files:**
- Modify: `cli/src/cli.ts`
- Modify: `cli/src/dependencies.ts`

- [ ] **Step 1: Update cli/src/dependencies.ts**

```typescript
import { Layer } from 'effect';
import { ExtractDatasetLive } from '#modules/extract-dataset/dependencies.ts';
import { TrainModelLive } from '#modules/train-model/dependencies.ts';
import { CheckStyleLive } from '#modules/check-style/dependencies.ts';
import { ExplainLive } from '#modules/explain/dependencies.ts';
import { RepoContextLive } from '#modules/repo-context/dependencies.ts';

export const AppLive = Layer.mergeAll(
  ExtractDatasetLive,
  TrainModelLive,
  CheckStyleLive,
  ExplainLive,
  RepoContextLive,
);
```

- [ ] **Step 2: Update cli/src/cli.ts** — import and register `status` and `list` commands, update help text

```typescript
import { Command } from 'effect/unstable/cli';
import { BunRuntime, BunServices } from '@effect/platform-bun';
import { Console, Effect } from 'effect';
import { extractCommand } from '#shell/infrastructure/adapters/in/commands/extract.command.ts';
import { trainCommand } from '#shell/infrastructure/adapters/in/commands/train.command.ts';
import { checkCommand } from '#shell/infrastructure/adapters/in/commands/check.command.ts';
import { explainCommand } from '#shell/infrastructure/adapters/in/commands/explain.command.ts';
import { updateCommand } from '#shell/infrastructure/adapters/in/commands/update.command.ts';
import { statusCommand } from '#shell/infrastructure/adapters/in/commands/status.command.ts';
import { listCommand } from '#shell/infrastructure/adapters/in/commands/list.command.ts';
import { AppLive } from '#dependencies';
import { version } from './version.ts';
import { updateNotify } from './update-notify.ts';

const app = Command.make('argot', {}, () =>
  Console.log(`argot v${version}

COMMANDS
  extract   Extract dataset from the current git repository
  train     Train a style model on the extracted dataset
  check     Check code against the trained style model
  explain   Explain style anomalies in detail
  status    Show current repository's argot state
  list      List all registered repositories
  update    Update the argot CLI

Run \`argot <command> --help\` for more information.`),
).pipe(
  Command.withSubcommands([
    extractCommand,
    trainCommand,
    checkCommand,
    explainCommand,
    statusCommand,
    listCommand,
    updateCommand,
  ]),
);

const program = Command.run(app, { version });
program.pipe(
  Effect.andThen(() => updateNotify),
  Effect.provide(AppLive),
  Effect.provide(BunServices.layer),
  BunRuntime.runMain,
);
```

- [ ] **Step 3: Verify TypeScript — full clean check**

```bash
cd cli && bunx tsgo --noEmit 2>&1
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add cli/src/cli.ts cli/src/dependencies.ts
git commit -m "feat: register status/list commands and wire RepoContextLive into AppLive"
```

---

## Task 18: Run full verify suite

**Files:** none modified — verification only.

- [ ] **Step 1: Run full verify**

```bash
just verify
```
Expected: all checks pass (lint, format, typecheck, boundaries, knip, tests).

- [ ] **Step 2: Run smoke test — argot status in this repo**

```bash
cd /path/to/argot && ./cli/src/cli.ts status
```
Expected: prints Repo, Dataset, Model, Settings lines.

- [ ] **Step 3: Run smoke test — argot list**

```bash
./cli/src/cli.ts list
```
Expected: prints table with at least the argot repo starred as current.

- [ ] **Step 4: Run smoke test — argot check with no flags**

```bash
./cli/src/cli.ts check
```
Expected: runs check against uncommitted changes using `.argot/model.pkl` from git root.

- [ ] **Step 5: Fix any issues found, commit fixes**

```bash
just verify
git add -p
git commit -m "fix: address verify failures from multi-repo UX changes"
```
