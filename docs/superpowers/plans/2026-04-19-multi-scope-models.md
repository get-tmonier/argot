# multi-scope models Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a single repo train/check multiple models, one per path-scope, so a polyglot codebase (e.g. `cli/` TS vs `engine/` Python) gets a clean style signal per scope. `argot extract` / `train` / `check` / `status` all auto-route through scopes defined in `.argot/config.json`. No config → behave exactly as today.

**Architecture:** A new `scopes` domain in `repo-context` owns config parsing and longest-prefix path routing. `ResolvedContext` grows a `scopes: ResolvedScope[]` list where each scope carries its own `datasetPath` + `modelPath`. The engine's `extract` and `check` gain a `--path-prefix` filter that is applied inside the inner commit/file loop. Each CLI command iterates scopes (or filters to `--scope <name>`) and invokes the engine once per scope. `check` maps each hunk to the longest-prefix-matching scope before dispatch.

**Tech Stack:** TypeScript/Bun (Effect v4 CLI, valibot), Python 3.13 (argparse, pygit2), Python test: pytest, TS test: bun test. `just verify` is the gate.

---

## File map

| File | Change |
|------|--------|
| `engine/argot/extract.py` | Add `--path-prefix` arg; skip records whose `file_path` doesn't start with prefix |
| `engine/argot/check.py` | Add `--path-prefix` arg; filter patches by prefix before scoring |
| `engine/argot/tests/test_extract_smoke.py` | Add `test_smoke_extract_with_path_prefix` |
| `engine/argot/tests/test_check_smoke.py` | Add `test_check_path_prefix_filters_patches` |
| `cli/src/modules/repo-context/domain/scopes.ts` | **New**: `ScopeConfig`, `ResolvedScope`, `pickScope`, `resolveScopes` |
| `cli/src/modules/repo-context/domain/scopes.test.ts` | **New**: tests for `pickScope` + `resolveScopes` |
| `cli/src/modules/repo-context/domain/repo-context.ts` | Add `scopes: ResolvedScope[]` to `ResolvedContext`; extend `RepoStatus` with `scopes?: ScopeStatus[]` |
| `cli/src/modules/repo-context/domain/errors.ts` | Add `ScopeConfigInvalid`, `ScopeNotFound` errors |
| `cli/src/modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts` | Read `.argot/config.json`, resolve scopes, populate `ResolvedContext.scopes`; `listRepos` populates per-scope stats |
| `cli/src/modules/extract-dataset/domain/extract-options.ts` | Add optional `pathPrefix` |
| `cli/src/modules/extract-dataset/application/ports/out/engine-runner.port.ts` | Accept `pathPrefix?` in `runExtract` |
| `cli/src/modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts` | Forward `pathPrefix` to adapter |
| `cli/src/modules/extract-dataset/infrastructure/adapters/out/bun-engine-runner.adapter.ts` | Append `--path-prefix <prefix>` when provided |
| `cli/src/modules/check-style/application/ports/out/style-checker.port.ts` | Accept `pathPrefix?` in `runCheck` |
| `cli/src/modules/check-style/application/use-cases/check-style.use-case.ts` | Forward `pathPrefix` |
| `cli/src/modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts` | Append `--path-prefix` when provided |
| `cli/src/shell/infrastructure/adapters/in/commands/extract.command.ts` | Iterate scopes; add `--scope <name>` option |
| `cli/src/shell/infrastructure/adapters/in/commands/train.command.ts` | Iterate scopes; add `--scope <name>` option |
| `cli/src/shell/infrastructure/adapters/in/commands/check.command.ts` | Iterate scopes, pass `--path-prefix` |
| `cli/src/shell/infrastructure/adapters/in/commands/status.command.ts` | Render per-scope stats when multi-scope |

---

## Invariants (applies to every task)

- **Backwards compat**: when `.argot/config.json` is missing, `ResolvedContext.scopes` contains exactly one `ResolvedScope { name: 'default', pathPrefix: '', datasetPath: '<gitRoot>/.argot/dataset.jsonl', modelPath: '<gitRoot>/.argot/model.pkl' }`. All external behaviour (file paths on disk, CLI output) is byte-identical to pre-change.
- **Path prefix format**: stored exactly as written in config (e.g. `cli/`). `pickScope` uses `file_path.startsWith(prefix)` (no normalization). Empty prefix matches everything and sits at the bottom of the longest-prefix priority.
- **Single-scope wire format**: when `scopes.length === 1` and `scopes[0].pathPrefix === ''`, DO NOT pass `--path-prefix ''` to the engine (keeps argv identical for the no-config path).

---

## Task 1: Engine — `extract.py --path-prefix` failing test

**Files:**
- Modify: `engine/argot/tests/test_extract_smoke.py`

- [ ] **Step 1: Add failing test**

Append at the bottom of `engine/argot/tests/test_extract_smoke.py`:

```python
def test_smoke_extract_with_path_prefix(tmp_path: Path) -> None:
    """--path-prefix filters output to records whose file_path starts with the prefix."""
    out = tmp_path / "dataset.jsonl"
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "argot.extract",
            str(REPO_ROOT),
            "--out",
            str(out),
            "--path-prefix",
            "engine/",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode in (0, 2), f"stderr: {result.stderr}"
    if result.returncode == 0:
        lines = out.read_text().strip().splitlines()
        assert len(lines) >= 1
        for line in lines:
            record = json.loads(line)
            assert record["file_path"].startswith("engine/"), (
                f"expected file_path under 'engine/', got {record['file_path']}"
            )
```

- [ ] **Step 2: Run test to verify it fails**

```bash
uv run pytest engine/argot/tests/test_extract_smoke.py::test_smoke_extract_with_path_prefix -v
```

Expected: FAIL with `error: unrecognized arguments: --path-prefix engine/` (extract.py rejects the unknown flag).

- [ ] **Step 3: Commit**

```bash
git add engine/argot/tests/test_extract_smoke.py
git commit -m "test(engine): failing test for extract --path-prefix filter"
```

---

## Task 2: Engine — implement `extract.py --path-prefix`

**Files:**
- Modify: `engine/argot/extract.py`

- [ ] **Step 1: Add CLI arg**

In `engine/argot/extract.py`, after the existing `--repo-name` argument block (ends line 41), insert before `args = parser.parse_args()` (line 42):

```python
    parser.add_argument(
        "--path-prefix",
        default=None,
        help="Only emit records whose file_path starts with this prefix (e.g. 'cli/')",
    )
```

- [ ] **Step 2: Apply filter inside walk loop**

In `engine/argot/extract.py`, inside `main()` just after the `walk_repo` for-loop header (currently `for commit, file_path, post_blob, hunks in walk_repo(repo_path):` at line 58) and before `lang = language_for_path(file_path)` (line 59), insert:

```python
            if args.path_prefix is not None and not file_path.startswith(args.path_prefix):
                continue
```

- [ ] **Step 3: Run the failing test**

```bash
uv run pytest engine/argot/tests/test_extract_smoke.py::test_smoke_extract_with_path_prefix -v
```

Expected: PASS.

- [ ] **Step 4: Run the full extract test file**

```bash
uv run pytest engine/argot/tests/test_extract_smoke.py -v
```

Expected: all 4 tests PASS (3 pre-existing + new one).

- [ ] **Step 5: Commit**

```bash
git add engine/argot/extract.py
git commit -m "feat(engine): add --path-prefix filter to extract"
```

---

## Task 3: Engine — `check.py --path-prefix` failing test

**Files:**
- Modify: `engine/argot/tests/test_check_smoke.py`

- [ ] **Step 1: Add failing test**

Append at the bottom of `engine/argot/tests/test_check_smoke.py`:

```python
def test_check_path_prefix_filters_patches(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """--path-prefix=other/ on a workdir change under main.py skips the hunk → exit 0."""
    _make_repo(tmp_path)
    model_path = _save_model(tmp_path)

    (tmp_path / "main.py").write_text("x = 1\ny = 2\nz = 3\n")

    monkeypatch.setattr(
        sys,
        "argv",
        [
            "argot-check",
            str(tmp_path),
            "",
            "--model",
            str(model_path),
            "--threshold",
            "-1.0",
            "--path-prefix",
            "other/",
        ],
    )
    with pytest.raises(SystemExit) as exc:
        check_mod.main()
    # prefix filters out the hunk → no violations → exit 0
    assert exc.value.code == 0
```

- [ ] **Step 2: Run test to verify it fails**

```bash
uv run pytest engine/argot/tests/test_check_smoke.py::test_check_path_prefix_filters_patches -v
```

Expected: FAIL with `error: unrecognized arguments: --path-prefix other/`.

- [ ] **Step 3: Commit**

```bash
git add engine/argot/tests/test_check_smoke.py
git commit -m "test(engine): failing test for check --path-prefix filter"
```

---

## Task 4: Engine — implement `check.py --path-prefix`

**Files:**
- Modify: `engine/argot/check.py`

- [ ] **Step 1: Add CLI arg**

In `engine/argot/check.py`, inside `main()` after the existing `--threshold` argument (line 126), insert before `args = parser.parse_args()` (line 127):

```python
    parser.add_argument(
        "--path-prefix",
        default=None,
        help="Only score hunks whose file_path starts with this prefix (e.g. 'cli/')",
    )
```

- [ ] **Step 2: Apply filter inside `_score_patches`**

In `engine/argot/check.py`, change the `_score_patches` signature (currently line 73–78) and its inner loop to accept an optional prefix:

Replace:

```python
def _score_patches(
    patches: Iterator[tuple[str, bytes, list[pygit2.DiffHunk]]],
    vectorizer: Any,
    model: JEPAArgot,
    label: str,
) -> tuple[list[tuple[float, str, int, str]], int]:
    """Score hunk patches; returns (results, total_hunk_count)."""
    context_lines = 50
    results: list[tuple[float, str, int, str]] = []
    hunk_count = 0

    with torch.no_grad():
        for file_path, post_blob, hunks in patches:
            lang = language_for_path(file_path)
            if lang is None:
                continue
```

With:

```python
def _score_patches(
    patches: Iterator[tuple[str, bytes, list[pygit2.DiffHunk]]],
    vectorizer: Any,
    model: JEPAArgot,
    label: str,
    path_prefix: str | None = None,
) -> tuple[list[tuple[float, str, int, str]], int]:
    """Score hunk patches; returns (results, total_hunk_count)."""
    context_lines = 50
    results: list[tuple[float, str, int, str]] = []
    hunk_count = 0

    with torch.no_grad():
        for file_path, post_blob, hunks in patches:
            if path_prefix is not None and not file_path.startswith(path_prefix):
                continue
            lang = language_for_path(file_path)
            if lang is None:
                continue
```

- [ ] **Step 3: Thread `path_prefix` into the single call site**

In `engine/argot/check.py`, change the call at line 167 from:

```python
    results, hunk_count = _score_patches(patches, vectorizer, model, context_label)
```

to:

```python
    results, hunk_count = _score_patches(
        patches, vectorizer, model, context_label, path_prefix=args.path_prefix
    )
```

- [ ] **Step 4: Run failing test**

```bash
uv run pytest engine/argot/tests/test_check_smoke.py::test_check_path_prefix_filters_patches -v
```

Expected: PASS.

- [ ] **Step 5: Run the full check test file**

```bash
uv run pytest engine/argot/tests/test_check_smoke.py engine/argot/tests/test_check.py -v
```

Expected: all PASS (pre-existing 2 in smoke, pre-existing ones in `test_check.py`, plus new one).

- [ ] **Step 6: Commit**

```bash
git add engine/argot/check.py
git commit -m "feat(engine): add --path-prefix filter to check"
```

---

## Task 5: CLI domain — `ScopeConfig` + `pickScope` (failing tests)

**Files:**
- Create: `cli/src/modules/repo-context/domain/scopes.test.ts`

- [ ] **Step 1: Write failing tests**

Create `cli/src/modules/repo-context/domain/scopes.test.ts`:

```typescript
import { describe, expect, it } from 'bun:test';
import { pickScope, resolveScopes, DEFAULT_SCOPE_NAME } from './scopes.ts';

describe('pickScope', () => {
  it('returns the only scope when a single default scope exists', () => {
    const scopes = resolveScopes('/repo', undefined);
    expect(pickScope(scopes, 'anywhere/foo.ts')!.name).toBe(DEFAULT_SCOPE_NAME);
  });

  it('matches by path prefix', () => {
    const scopes = resolveScopes('/repo', {
      scopes: [
        { name: 'cli', path: 'cli/' },
        { name: 'engine', path: 'engine/' },
      ],
    });
    expect(pickScope(scopes, 'cli/src/foo.ts')!.name).toBe('cli');
    expect(pickScope(scopes, 'engine/argot/bar.py')!.name).toBe('engine');
  });

  it('uses longest-prefix match when prefixes nest', () => {
    const scopes = resolveScopes('/repo', {
      scopes: [
        { name: 'inner', path: 'src/core/' },
        { name: 'outer', path: 'src/' },
      ],
    });
    expect(pickScope(scopes, 'src/core/x.ts')!.name).toBe('inner');
    expect(pickScope(scopes, 'src/util.ts')!.name).toBe('outer');
  });

  it('returns null when no scope matches', () => {
    const scopes = resolveScopes('/repo', {
      scopes: [{ name: 'cli', path: 'cli/' }],
    });
    expect(pickScope(scopes, 'engine/x.py')).toBeNull();
  });
});

describe('resolveScopes', () => {
  it('returns a single default scope when config is missing', () => {
    const scopes = resolveScopes('/repo', undefined);
    expect(scopes).toEqual([
      {
        name: 'default',
        pathPrefix: '',
        datasetPath: '/repo/.argot/dataset.jsonl',
        modelPath: '/repo/.argot/model.pkl',
      },
    ]);
  });

  it('builds per-scope paths under .argot/models/<name>/', () => {
    const scopes = resolveScopes('/repo', {
      scopes: [
        { name: 'cli', path: 'cli/' },
        { name: 'engine', path: 'engine/' },
      ],
    });
    expect(scopes).toEqual([
      {
        name: 'cli',
        pathPrefix: 'cli/',
        datasetPath: '/repo/.argot/models/cli/dataset.jsonl',
        modelPath: '/repo/.argot/models/cli/model.pkl',
      },
      {
        name: 'engine',
        pathPrefix: 'engine/',
        datasetPath: '/repo/.argot/models/engine/dataset.jsonl',
        modelPath: '/repo/.argot/models/engine/model.pkl',
      },
    ]);
  });

  it('throws on duplicate scope names', () => {
    expect(() =>
      resolveScopes('/repo', {
        scopes: [
          { name: 'x', path: 'a/' },
          { name: 'x', path: 'b/' },
        ],
      }),
    ).toThrow(/duplicate scope name/i);
  });

  it('throws on empty scopes array', () => {
    expect(() => resolveScopes('/repo', { scopes: [] })).toThrow(/at least one scope/i);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bun test --cwd cli cli/src/modules/repo-context/domain/scopes.test.ts
```

Expected: FAIL with `Cannot find module './scopes.ts'`.

- [ ] **Step 3: Commit**

```bash
git add cli/src/modules/repo-context/domain/scopes.test.ts
git commit -m "test(cli): failing tests for scopes domain"
```

---

## Task 6: CLI domain — implement `scopes.ts`

**Files:**
- Create: `cli/src/modules/repo-context/domain/scopes.ts`

- [ ] **Step 1: Write the module**

Create `cli/src/modules/repo-context/domain/scopes.ts`:

```typescript
import { join } from 'node:path';
import * as v from 'valibot';

export const DEFAULT_SCOPE_NAME = 'default';

export const ScopeConfigSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1)),
  path: v.string(),
});
export type ScopeConfig = v.InferOutput<typeof ScopeConfigSchema>;

export const ScopesFileSchema = v.object({
  scopes: v.array(ScopeConfigSchema),
});
export type ScopesFile = v.InferOutput<typeof ScopesFileSchema>;

export interface ResolvedScope {
  name: string;
  pathPrefix: string;
  datasetPath: string;
  modelPath: string;
}

export function resolveScopes(
  gitRoot: string,
  config: ScopesFile | undefined,
): ResolvedScope[] {
  if (!config) {
    return [
      {
        name: DEFAULT_SCOPE_NAME,
        pathPrefix: '',
        datasetPath: join(gitRoot, '.argot', 'dataset.jsonl'),
        modelPath: join(gitRoot, '.argot', 'model.pkl'),
      },
    ];
  }

  if (config.scopes.length === 0) {
    throw new Error('config.json must list at least one scope');
  }

  const seen = new Set<string>();
  for (const s of config.scopes) {
    if (seen.has(s.name)) {
      throw new Error(`duplicate scope name: ${s.name}`);
    }
    seen.add(s.name);
  }

  return config.scopes.map((s) => ({
    name: s.name,
    pathPrefix: s.path,
    datasetPath: join(gitRoot, '.argot', 'models', s.name, 'dataset.jsonl'),
    modelPath: join(gitRoot, '.argot', 'models', s.name, 'model.pkl'),
  }));
}

export function pickScope(
  scopes: readonly ResolvedScope[],
  filePath: string,
): ResolvedScope | null {
  let best: ResolvedScope | null = null;
  for (const s of scopes) {
    if (!filePath.startsWith(s.pathPrefix)) continue;
    if (best === null || s.pathPrefix.length > best.pathPrefix.length) {
      best = s;
    }
  }
  return best;
}
```

- [ ] **Step 2: Run test to verify it passes**

```bash
bun test --cwd cli cli/src/modules/repo-context/domain/scopes.test.ts
```

Expected: all 7 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add cli/src/modules/repo-context/domain/scopes.ts
git commit -m "feat(cli): scope config domain — pickScope + resolveScopes"
```

---

## Task 7: CLI domain — extend `ResolvedContext`, `RepoStatus`, errors

**Files:**
- Modify: `cli/src/modules/repo-context/domain/repo-context.ts`
- Modify: `cli/src/modules/repo-context/domain/errors.ts`

- [ ] **Step 1: Extend `ResolvedContext` and add `ScopeStatus`**

Replace the whole contents of `cli/src/modules/repo-context/domain/repo-context.ts` with:

```typescript
import type { Preferences } from './settings.ts';
import type { ResolvedScope } from './scopes.ts';

export interface ResolvedContext {
  gitRoot: string;
  name: string;
  datasetPath: string;
  modelPath: string;
  scopes: ResolvedScope[];
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

export interface ScopeStatus {
  name: string;
  pathPrefix: string;
  datasetInfo: DatasetInfo | null;
  modelInfo: ModelInfo | null;
}

export interface RepoStatus {
  path: string;
  name: string;
  isCurrent: boolean;
  datasetInfo: DatasetInfo | null;
  modelInfo: ModelInfo | null;
  scopes: ScopeStatus[];
}
```

Note: `datasetPath` and `modelPath` stay on `ResolvedContext` so existing single-scope callers keep working; they now point at `scopes[0]`'s paths. In a multi-scope config, these fields mirror the first scope (callers must iterate `scopes` explicitly — we'll update all commands in later tasks).

- [ ] **Step 2: Read current errors.ts**

Look at `cli/src/modules/repo-context/domain/errors.ts` to preserve existing error classes.

- [ ] **Step 3: Add scope-related errors**

Append to `cli/src/modules/repo-context/domain/errors.ts` (keep existing exports):

```typescript
import { Data } from 'effect';

export class ScopeConfigInvalid extends Data.TaggedError('ScopeConfigInvalid')<{
  readonly cause: unknown;
}> {}

export class ScopeNotFound extends Data.TaggedError('ScopeNotFound')<{
  readonly name: string;
  readonly available: readonly string[];
}> {}
```

If `import { Data } from 'effect'` is already present, don't duplicate the import.

Then update the `RepoContextError` union in the same file to include the new errors. Look at the existing union (e.g. `export type RepoContextError = SettingsReadError | ...`) and add `| ScopeConfigInvalid | ScopeNotFound`.

- [ ] **Step 4: Run type check**

```bash
bun x tsgo --cwd cli --noEmit
```

Expected: PASS. (Any failing consumers will be fixed in Tasks 8–12.)

If type errors surface in other modules due to the new required `scopes` field, they will be addressed in the next task (`fs-repo-context.adapter.ts`). If the adapter itself doesn't yet populate `scopes`, that's fine for the intermediate commit — it's the next task.

- [ ] **Step 5: Commit**

```bash
git add cli/src/modules/repo-context/domain/repo-context.ts cli/src/modules/repo-context/domain/errors.ts
git commit -m "feat(cli): extend ResolvedContext with scopes + new error types"
```

---

## Task 8: CLI adapter — read `.argot/config.json`, populate `scopes`

**Files:**
- Modify: `cli/src/modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts`

- [ ] **Step 1: Add config-reader helper**

In `cli/src/modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts`, add new imports at the top alongside the existing ones:

```typescript
import * as v from 'valibot';
import { resolveScopes, ScopesFileSchema } from '#modules/repo-context/domain/scopes.ts';
import { ScopeConfigInvalid } from '#modules/repo-context/domain/errors.ts';
import type { ResolvedScope } from '#modules/repo-context/domain/scopes.ts';
```

Add the reader after `readLocalSettingsAsync` (around line 57):

```typescript
async function readScopesConfigAsync(gitRoot: string): Promise<ResolvedScope[]> {
  const configPath = join(gitRoot, '.argot', 'config.json');
  let raw: string;
  try {
    raw = await readFile(configPath, 'utf-8');
  } catch {
    return resolveScopes(gitRoot, undefined);
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (cause) {
    throw new ScopeConfigInvalid({ cause });
  }

  const result = v.safeParse(ScopesFileSchema, parsed);
  if (!result.success) {
    throw new ScopeConfigInvalid({ cause: result.issues });
  }

  try {
    return resolveScopes(gitRoot, result.output);
  } catch (cause) {
    throw new ScopeConfigInvalid({ cause });
  }
}
```

- [ ] **Step 2: Wire it into `resolveContext`**

In `resolveContext()`'s `try` block, after the `preferences` line (around line 76) and before the `now` line:

```typescript
          const scopes = await readScopesConfigAsync(gitRoot);
```

Then change the returned object (lines 90–96) to:

```typescript
          return {
            gitRoot,
            name: global.repos[gitRoot]!.name,
            datasetPath: scopes[0]!.datasetPath,
            modelPath: scopes[0]!.modelPath,
            scopes,
            preferences,
          };
```

- [ ] **Step 3: Update `catch` in `resolveContext`**

The existing `catch` at the bottom of the Effect.tryPromise block re-raises as `SettingsReadError`. Replace it so `ScopeConfigInvalid` passes through:

```typescript
        catch: (e) => (e instanceof ScopeConfigInvalid ? e : new SettingsReadError({ cause: e })),
```

- [ ] **Step 4: Update `listRepos` to populate per-scope stats**

Replace the body of the `listRepos` for-loop (lines 108–120) with:

```typescript
          for (const [path, entry] of Object.entries(global.repos)) {
            let scopesResolved: ResolvedScope[];
            try {
              scopesResolved = await readScopesConfigAsync(path);
            } catch {
              scopesResolved = resolveScopes(path, undefined);
            }

            const scopeStatuses = await Promise.all(
              scopesResolved.map(async (s) => ({
                name: s.name,
                pathPrefix: s.pathPrefix,
                datasetInfo: await statOrNull(s.datasetPath),
                modelInfo: await statOrNull(s.modelPath),
              })),
            );

            const primary = scopeStatuses[0]!;
            results.push({
              path,
              name: entry.name,
              isCurrent: path === currentRoot,
              datasetInfo: primary.datasetInfo,
              modelInfo: primary.modelInfo,
              scopes: scopeStatuses,
            });
          }
```

- [ ] **Step 5: Verify type-check and tests**

```bash
bun x tsgo --cwd cli --noEmit && bun test --cwd cli
```

Expected: PASS. If existing callers fail because they destructure `ResolvedContext`, they will stay compiling because `datasetPath`/`modelPath` are still present; they keep single-scope semantics until later tasks.

- [ ] **Step 6: Commit**

```bash
git add cli/src/modules/repo-context/infrastructure/adapters/out/fs-repo-context.adapter.ts
git commit -m "feat(cli): read .argot/config.json and resolve scopes"
```

---

## Task 9: Extract path-prefix plumbing (port + adapter + use-case + options)

**Files:**
- Modify: `cli/src/modules/extract-dataset/domain/extract-options.ts`
- Modify: `cli/src/modules/extract-dataset/application/ports/out/engine-runner.port.ts`
- Modify: `cli/src/modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts`
- Modify: `cli/src/modules/extract-dataset/infrastructure/adapters/out/bun-engine-runner.adapter.ts`

- [ ] **Step 1: Extend `ExtractOptions`**

Replace the contents of `cli/src/modules/extract-dataset/domain/extract-options.ts`:

```typescript
import * as v from 'valibot';

export const ExtractOptionsSchema = v.object({
  repoPath: v.string(),
  outputPath: v.string(),
  pathPrefix: v.optional(v.string()),
});

export type ExtractOptions = v.InferOutput<typeof ExtractOptionsSchema>;
```

- [ ] **Step 2: Extend `EngineRunner` port**

Replace `cli/src/modules/extract-dataset/application/ports/out/engine-runner.port.ts`:

```typescript
import { Context } from 'effect';
import type { Effect } from 'effect';
import type { EngineError } from '#modules/extract-dataset/domain/errors.ts';

interface EngineRunnerShape {
  readonly runExtract: (args: {
    repoPath: string;
    outputPath: string;
    pathPrefix?: string;
  }) => Effect.Effect<void, EngineError>;
}

export class EngineRunner extends Context.Service<EngineRunner, EngineRunnerShape>()(
  '@argot/EngineRunner',
) {}
```

- [ ] **Step 3: Pass `pathPrefix` in use-case**

Replace `cli/src/modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts`:

```typescript
import { Effect } from 'effect';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import type { ExtractOptions } from '#modules/extract-dataset/domain/extract-options.ts';
import type { EngineError } from '#modules/extract-dataset/domain/errors.ts';

export const runExtractDataset = (
  opts: ExtractOptions,
): Effect.Effect<{ outputPath: string }, EngineError, EngineRunner> =>
  Effect.gen(function* () {
    const engineRunner = yield* EngineRunner;
    yield* engineRunner.runExtract({
      repoPath: opts.repoPath,
      outputPath: opts.outputPath,
      pathPrefix: opts.pathPrefix,
    });
    return { outputPath: opts.outputPath };
  });
```

- [ ] **Step 4: Append `--path-prefix` in adapter**

Replace `cli/src/modules/extract-dataset/infrastructure/adapters/out/bun-engine-runner.adapter.ts`:

```typescript
import { spawn } from 'node:child_process';
import { Effect, Layer } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import { EngineExitNonZero, EngineSpawnFailed } from '#modules/extract-dataset/domain/errors.ts';

export const BunEngineRunnerLive = Layer.effect(EngineRunner)(
  Effect.succeed({
    runExtract: ({
      repoPath,
      outputPath,
      pathPrefix,
    }: {
      repoPath: string;
      outputPath: string;
      pathPrefix?: string;
    }) =>
      Effect.callback<void, EngineExitNonZero | EngineSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.extract');
        const extraArgs: string[] = pathPrefix ? ['--path-prefix', pathPrefix] : [];
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(cmd, [...args, repoPath, '--out', outputPath, ...extraArgs], {
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

- [ ] **Step 5: Type check and tests**

```bash
bun x tsgo --cwd cli --noEmit && bun test --cwd cli
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add cli/src/modules/extract-dataset
git commit -m "feat(cli): plumb pathPrefix through extract-dataset module"
```

---

## Task 10: Check path-prefix plumbing (port + adapter + use-case)

**Files:**
- Modify: `cli/src/modules/check-style/application/ports/out/style-checker.port.ts`
- Modify: `cli/src/modules/check-style/application/use-cases/check-style.use-case.ts`
- Modify: `cli/src/modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts`

- [ ] **Step 1: Read current port**

Look at `cli/src/modules/check-style/application/ports/out/style-checker.port.ts` to confirm the existing signature (should be `runCheck: { repoPath, ref, modelPath, threshold } → Effect<boolean, CheckError>`).

- [ ] **Step 2: Add `pathPrefix?` to port**

Edit `cli/src/modules/check-style/application/ports/out/style-checker.port.ts`. Add `pathPrefix?: string;` to the `runCheck` args interface (keep the existing four fields).

- [ ] **Step 3: Forward in use-case**

Replace `cli/src/modules/check-style/application/use-cases/check-style.use-case.ts`:

```typescript
import { Effect } from 'effect';
import { StyleChecker } from '#modules/check-style/application/ports/out/style-checker.port.ts';
import type { CheckError } from '#modules/check-style/domain/errors.ts';

export const runCheckStyle = (args: {
  repoPath: string;
  ref: string;
  modelPath: string;
  threshold: number;
  pathPrefix?: string;
}): Effect.Effect<boolean, CheckError, StyleChecker> =>
  Effect.gen(function* () {
    const styleChecker = yield* StyleChecker;
    return yield* styleChecker.runCheck(args);
  });
```

- [ ] **Step 4: Append `--path-prefix` in adapter**

Replace `cli/src/modules/check-style/infrastructure/adapters/out/bun-style-checker.adapter.ts`:

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
      pathPrefix,
    }: {
      repoPath: string;
      ref: string;
      modelPath: string;
      threshold: number;
      pathPrefix?: string;
    }) =>
      Effect.callback<boolean, CheckExitNonZero | CheckSpawnFailed>((resume) => {
        const { cmd, args } = engineCmd('argot.check');
        const extraArgs: string[] = pathPrefix ? ['--path-prefix', pathPrefix] : [];
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(
            cmd,
            [
              ...args,
              repoPath,
              ref,
              '--model',
              modelPath,
              '--threshold',
              String(threshold),
              ...extraArgs,
            ],
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
            resume(Effect.succeed(false));
          } else if (code === 1) {
            resume(Effect.succeed(true));
          } else {
            const stderr = Buffer.concat(stderrChunks).toString('utf-8');
            resume(Effect.fail(new CheckExitNonZero({ exitCode: code ?? -1, stderr })));
          }
        });
      }),
  }),
);
```

- [ ] **Step 5: Type check and tests**

```bash
bun x tsgo --cwd cli --noEmit && bun test --cwd cli
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add cli/src/modules/check-style
git commit -m "feat(cli): plumb pathPrefix through check-style module"
```

---

## Task 11: Shell — extract command iterates scopes + supports `--scope`

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/extract.command.ts`

- [ ] **Step 1: Read the Effect CLI `Options.string` + `Options.withDefault` conventions**

Look at `cli/src/shell/infrastructure/adapters/in/commands/check.command.ts` for how optional arguments/options are declared (uses `Argument.string(...).pipe(Argument.withDefault(...))`). Options are declared similarly via `Options.string(...).pipe(Options.optional)` or equivalent. Check `cli/src/shell/infrastructure/adapters/in/commands/update.command.ts` (or any other command using options) to find the exact import path and idiom in this codebase — use the pattern found there.

- [ ] **Step 2: Rewrite extract.command.ts**

Replace `cli/src/shell/infrastructure/adapters/in/commands/extract.command.ts` with:

```typescript
import { Command, Options } from 'effect/unstable/cli';
import { Console, Effect, Option } from 'effect';
import { mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { ScopeNotFound } from '#modules/repo-context/domain/errors.ts';
import { runExtractDataset } from '#modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts';

export const extractCommand = Command.make(
  'extract',
  {
    scope: Options.string('scope').pipe(
      Options.optional,
      Options.withDescription('Only extract a single scope by name (multi-scope configs only)'),
    ),
  },
  ({ scope }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);

      const scopeName = Option.getOrNull(scope);
      const scopesToRun = scopeName
        ? ctx.scopes.filter((s) => s.name === scopeName)
        : ctx.scopes;

      if (scopeName && scopesToRun.length === 0) {
        return yield* Effect.fail(
          new ScopeNotFound({ name: scopeName, available: ctx.scopes.map((s) => s.name) }),
        );
      }

      for (const s of scopesToRun) {
        yield* Console.log(`→ scope ${s.name}${s.pathPrefix ? ` (${s.pathPrefix})` : ''}`);
        yield* Effect.tryPromise(() => mkdir(dirname(s.datasetPath), { recursive: true }));
        const result = yield* runExtractDataset({
          repoPath: ctx.gitRoot,
          outputPath: s.datasetPath,
          pathPrefix: s.pathPrefix === '' ? undefined : s.pathPrefix,
        });
        yield* Console.log(`  dataset → ${result.outputPath}`);
      }
    }),
);
```

If the `Options.string(...).pipe(Options.optional, Options.withDescription(...))` chain doesn't compile in this Effect version, replace with the exact idiom found in Step 1.

- [ ] **Step 3: Type check and tests**

```bash
bun x tsgo --cwd cli --noEmit && bun test --cwd cli
```

Expected: PASS.

- [ ] **Step 4: Smoke check the command builds**

```bash
ARGOT_DEV=1 bun run --cwd cli src/cli.ts extract --help
```

Expected: help text shows `--scope`. Command does not execute the engine (no subcommand run).

- [ ] **Step 5: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/extract.command.ts
git commit -m "feat(cli): extract iterates scopes with --scope selector"
```

---

## Task 12: Shell — train command iterates scopes + supports `--scope`

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/train.command.ts`

- [ ] **Step 1: Rewrite train.command.ts**

Replace `cli/src/shell/infrastructure/adapters/in/commands/train.command.ts` with:

```typescript
import { Command, Options } from 'effect/unstable/cli';
import { Console, Effect, Option } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { ScopeNotFound } from '#modules/repo-context/domain/errors.ts';
import { runTrainModel } from '#modules/train-model/application/use-cases/train-model.use-case.ts';

export const trainCommand = Command.make(
  'train',
  {
    scope: Options.string('scope').pipe(
      Options.optional,
      Options.withDescription('Only train a single scope by name (multi-scope configs only)'),
    ),
  },
  ({ scope }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);

      const scopeName = Option.getOrNull(scope);
      const scopesToRun = scopeName
        ? ctx.scopes.filter((s) => s.name === scopeName)
        : ctx.scopes;

      if (scopeName && scopesToRun.length === 0) {
        return yield* Effect.fail(
          new ScopeNotFound({ name: scopeName, available: ctx.scopes.map((s) => s.name) }),
        );
      }

      for (const s of scopesToRun) {
        yield* Console.log(`→ scope ${s.name}`);
        yield* runTrainModel({ datasetPath: s.datasetPath, modelPath: s.modelPath });
        yield* Console.log(`  model → ${s.modelPath}`);
      }
    }),
);
```

- [ ] **Step 2: Type check and tests**

```bash
bun x tsgo --cwd cli --noEmit && bun test --cwd cli
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/train.command.ts
git commit -m "feat(cli): train iterates scopes with --scope selector"
```

---

## Task 13: Shell — check command dispatches per scope

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/check.command.ts`

Strategy: for each scope, call `runCheckStyle` with `--path-prefix` set to the scope's `pathPrefix`, and `modelPath` set to the scope's model. Aggregate results with OR — if any scope reports violations, exit 1. For the backwards-compatible single-default-scope path, this reduces to a single call with no `--path-prefix`, byte-identical to pre-change.

- [ ] **Step 1: Rewrite check.command.ts**

Replace `cli/src/shell/infrastructure/adapters/in/commands/check.command.ts` with:

```typescript
import { Argument, Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { stat } from 'node:fs/promises';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runCheckStyle } from '#modules/check-style/application/use-cases/check-style.use-case.ts';

async function modelExists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

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
      yield* Console.log(
        `argot · ${ctx.name} (${ctx.gitRoot}) · threshold ${ctx.preferences.threshold}`,
      );

      let anyViolations = false;
      for (const s of ctx.scopes) {
        const hasModel = yield* Effect.tryPromise(() => modelExists(s.modelPath)).pipe(
          Effect.orElseSucceed(() => false),
        );
        if (!hasModel) {
          yield* Console.log(`scope ${s.name}: no model — run 'argot train --scope ${s.name}'`);
          continue;
        }

        if (ctx.scopes.length > 1) {
          yield* Console.log(
            `→ scope ${s.name}${s.pathPrefix ? ` (${s.pathPrefix})` : ''}`,
          );
        }

        const violations = yield* runCheckStyle({
          repoPath: ctx.gitRoot,
          ref,
          modelPath: s.modelPath,
          threshold: ctx.preferences.threshold,
          pathPrefix: s.pathPrefix === '' ? undefined : s.pathPrefix,
        });
        anyViolations = anyViolations || violations;
      }

      if (anyViolations) process.exit(1);
    }),
);
```

- [ ] **Step 2: Type check and tests**

```bash
bun x tsgo --cwd cli --noEmit && bun test --cwd cli
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/check.command.ts
git commit -m "feat(cli): check dispatches per scope with path-prefix filter"
```

---

## Task 14: Shell — status command renders per-scope stats

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/status.command.ts`

- [ ] **Step 1: Rewrite status.command.ts**

Replace `cli/src/shell/infrastructure/adapters/in/commands/status.command.ts` with:

```typescript
import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { stat } from 'node:fs/promises';
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

async function datasetLine(path: string): Promise<string> {
  try {
    const s = await stat(path);
    const count = await countJsonlLines(path);
    const countStr = count !== null ? `${count} records · ` : '';
    return `${countStr}${formatBytes(s.size)} · last extracted ${formatAge(s.mtime)}`;
  } catch {
    return '—';
  }
}

async function modelLine(path: string): Promise<string> {
  try {
    const s = await stat(path);
    return `trained ${formatAge(s.mtime)} · ${formatBytes(s.size)}`;
  } catch {
    return 'not trained';
  }
}

export const statusCommand = Command.make('status', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();

    yield* Console.log(`Repo:     ${ctx.name} (${ctx.gitRoot})`);

    const multi = ctx.scopes.length > 1 || ctx.scopes[0]!.pathPrefix !== '';

    if (!multi) {
      const s = ctx.scopes[0]!;
      const ds = yield* Effect.promise(() => datasetLine(s.datasetPath));
      const md = yield* Effect.promise(() => modelLine(s.modelPath));
      yield* Console.log(`Dataset:  ${ds}`);
      yield* Console.log(`Model:    ${md}`);
    } else {
      yield* Console.log(`Scopes:   ${ctx.scopes.length}`);
      for (const s of ctx.scopes) {
        const ds = yield* Effect.promise(() => datasetLine(s.datasetPath));
        const md = yield* Effect.promise(() => modelLine(s.modelPath));
        yield* Console.log(`  ${s.name} (${s.pathPrefix})`);
        yield* Console.log(`    dataset: ${ds}`);
        yield* Console.log(`    model:   ${md}`);
      }
    }

    const thresholdSource =
      ctx.preferences.threshold === 0.5 ? '(global default)' : '(local override)';
    yield* Console.log(
      `Settings: threshold ${ctx.preferences.threshold} ${thresholdSource} · model ${ctx.preferences.model}`,
    );
  }),
);
```

- [ ] **Step 2: Type check and tests**

```bash
bun x tsgo --cwd cli --noEmit && bun test --cwd cli
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/status.command.ts
git commit -m "feat(cli): status renders per-scope stats in multi-scope repos"
```

---

## Task 15: Integration smoke — real argot repo two-scope flow

**Files:**
- Create: `cli/src/shell/infrastructure/adapters/in/commands/multi-scope.integration.test.ts`

This test drives the real engine end-to-end on a temporary git repo with two files (a `cli/foo.ts` and an `engine/bar.py`) across a few commits, using a `.argot/config.json` that defines two scopes. We verify that after extract each scope's `dataset.jsonl` contains only records matching its prefix.

- [ ] **Step 1: Read existing test patterns**

Look at `cli/src/cli.test.ts` and any file matching `*.integration.test.ts` under `cli/src/` to understand how integration tests invoke the CLI (typically via `spawn` or by running `bun src/cli.ts` with `ARGOT_DEV=1`). Reuse that pattern rather than inventing a new one.

- [ ] **Step 2: Write the integration test**

If the existing pattern uses `spawn` directly, create `cli/src/shell/infrastructure/adapters/in/commands/multi-scope.integration.test.ts`:

```typescript
import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { mkdtemp, rm, writeFile, mkdir, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

let workDir: string;

async function run(cmd: string, args: string[], cwd: string): Promise<{ code: number; stderr: string; stdout: string }> {
  const result = spawnSync(cmd, args, { cwd, encoding: 'utf-8' });
  return {
    code: result.status ?? -1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

async function git(args: string[]): Promise<void> {
  const r = await run('git', args, workDir);
  if (r.code !== 0) throw new Error(`git ${args.join(' ')} → ${r.code}: ${r.stderr}`);
}

beforeAll(async () => {
  workDir = await mkdtemp(join(tmpdir(), 'argot-multi-'));

  await git(['init', '--initial-branch=main']);
  await git(['config', 'user.email', 'test@example.com']);
  await git(['config', 'user.name', 'Test']);

  await mkdir(join(workDir, 'cli'), { recursive: true });
  await mkdir(join(workDir, 'engine'), { recursive: true });

  await writeFile(join(workDir, 'cli', 'foo.ts'), 'export const x = 1;\n');
  await writeFile(join(workDir, 'engine', 'bar.py'), 'x = 1\n');
  await git(['add', '.']);
  await git(['commit', '-m', 'init']);

  await writeFile(join(workDir, 'cli', 'foo.ts'), 'export const x = 1;\nexport const y = 2;\n');
  await writeFile(join(workDir, 'engine', 'bar.py'), 'x = 1\ny = 2\n');
  await git(['commit', '-am', 'add y']);

  await mkdir(join(workDir, '.argot'), { recursive: true });
  await writeFile(
    join(workDir, '.argot', 'config.json'),
    JSON.stringify({
      scopes: [
        { name: 'cli', path: 'cli/' },
        { name: 'engine', path: 'engine/' },
      ],
    }),
  );
});

afterAll(async () => {
  await rm(workDir, { recursive: true, force: true });
});

describe('multi-scope extract', () => {
  it('writes one dataset per scope, each containing only records under its prefix', async () => {
    const cliEntry = join(import.meta.dir, '..', '..', '..', '..', 'cli.ts');
    const r = await run('bun', ['run', cliEntry, 'extract'], workDir);
    expect(r.code).toBe(0);

    const cliData = await readFile(join(workDir, '.argot/models/cli/dataset.jsonl'), 'utf-8');
    const engineData = await readFile(join(workDir, '.argot/models/engine/dataset.jsonl'), 'utf-8');

    const cliLines = cliData.trim().split('\n').filter(Boolean);
    const engineLines = engineData.trim().split('\n').filter(Boolean);

    expect(cliLines.length).toBeGreaterThan(0);
    expect(engineLines.length).toBeGreaterThan(0);

    for (const line of cliLines) {
      const r = JSON.parse(line);
      expect(r.file_path.startsWith('cli/')).toBe(true);
    }
    for (const line of engineLines) {
      const r = JSON.parse(line);
      expect(r.file_path.startsWith('engine/')).toBe(true);
    }
  }, 120_000);
});
```

The spawned command needs the engine available. In dev mode, set `ARGOT_DEV=1`. Adjust the `run('bun', ...)` call to inject env:

```typescript
const result = spawnSync(cmd, args, { cwd, encoding: 'utf-8', env: { ...process.env, ARGOT_DEV: '1' } });
```

Replace the `run` implementation accordingly.

- [ ] **Step 3: Run the integration test**

```bash
bun test --cwd cli cli/src/shell/infrastructure/adapters/in/commands/multi-scope.integration.test.ts
```

Expected: PASS.

If this test is too slow for the normal suite, tag it by file naming (e.g. `*.integration.test.ts`) and leave a note; otherwise include it in the default suite.

- [ ] **Step 4: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/multi-scope.integration.test.ts
git commit -m "test(cli): integration — two-scope extract writes prefix-filtered datasets"
```

---

## Task 16: No-config backwards-compatibility smoke

**Files:**
- Create: `cli/src/shell/infrastructure/adapters/in/commands/single-scope.integration.test.ts`

Goal: verify that when no `.argot/config.json` is present, `extract` still writes to `.argot/dataset.jsonl` (single file, default path) and `check` still invokes the engine without `--path-prefix`.

- [ ] **Step 1: Write the test**

Create `cli/src/shell/infrastructure/adapters/in/commands/single-scope.integration.test.ts`:

```typescript
import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { access, constants, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

let workDir: string;

function run(cmd: string, args: string[], cwd: string): { code: number; stderr: string; stdout: string } {
  const result = spawnSync(cmd, args, {
    cwd,
    encoding: 'utf-8',
    env: { ...process.env, ARGOT_DEV: '1' },
  });
  return {
    code: result.status ?? -1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

function git(args: string[]): void {
  const r = run('git', args, workDir);
  if (r.code !== 0) throw new Error(`git ${args.join(' ')} → ${r.code}: ${r.stderr}`);
}

beforeAll(async () => {
  workDir = await mkdtemp(join(tmpdir(), 'argot-single-'));
  git(['init', '--initial-branch=main']);
  git(['config', 'user.email', 'test@example.com']);
  git(['config', 'user.name', 'Test']);
  await mkdir(join(workDir, 'src'), { recursive: true });
  await writeFile(join(workDir, 'src', 'foo.ts'), 'export const x = 1;\n');
  git(['add', '.']);
  git(['commit', '-m', 'init']);
  await writeFile(join(workDir, 'src', 'foo.ts'), 'export const x = 1;\nexport const y = 2;\n');
  git(['commit', '-am', 'update']);
});

afterAll(async () => {
  await rm(workDir, { recursive: true, force: true });
});

describe('no config (single default scope)', () => {
  it('writes dataset to .argot/dataset.jsonl — no models/ subdirectory', async () => {
    const cliEntry = join(import.meta.dir, '..', '..', '..', '..', 'cli.ts');
    const r = run('bun', ['run', cliEntry, 'extract'], workDir);
    expect(r.code).toBe(0);

    await access(join(workDir, '.argot', 'dataset.jsonl'), constants.F_OK);
    await expect(access(join(workDir, '.argot', 'models'), constants.F_OK)).rejects.toThrow();
  }, 120_000);
});
```

- [ ] **Step 2: Run the test**

```bash
bun test --cwd cli cli/src/shell/infrastructure/adapters/in/commands/single-scope.integration.test.ts
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/single-scope.integration.test.ts
git commit -m "test(cli): integration — no config preserves legacy single-dataset layout"
```

---

## Task 17: Final verify + clean up + documentation

**Files:**
- Possibly: `cli/.dependency-cruiser.cjs` (only if a rule trips)
- Possibly: `README.md` / `cli/README.md` (update if user-facing docs exist)

- [ ] **Step 1: Run full verify**

```bash
just verify
```

Expected: PASS. If dependency-cruiser complains about `cli/src/shell/.../commands/*.ts` importing `#modules/repo-context/domain/errors.ts`, that import pattern is already used elsewhere (shell commands pull domain types), so a fresh violation is unlikely. If it does flag something, address the specific rule rather than loosening global config.

- [ ] **Step 2: Fix any knip / unused-export warnings**

If knip flags unused exports from `scopes.ts` (for example `DEFAULT_SCOPE_NAME`, `ScopesFileSchema`, or types only consumed within the same file), remove them or mark them used by the adapter. Do not blanket-suppress.

- [ ] **Step 3: Update README only if multi-scope docs already exist**

Look for existing `multi-repo` or `config.json` references in `README.md` / `cli/README.md`. Add a short "Multi-scope models" section with a config example ONLY if there's already a configuration section. Otherwise skip — the user hasn't asked for new docs.

- [ ] **Step 4: Final run**

```bash
just verify && just test
```

Expected: PASS across bun + pytest.

- [ ] **Step 5: Commit any trailing fixes**

```bash
git status
git add <any fixup files>
git commit -m "chore: address verify feedback for multi-scope models"
```

Only commit if `just verify` required a fix. If clean, skip this step.

---

## Self-review checklist (run before handing off)

1. **Spec coverage**
   - [x] `argot extract .` with config.json runs all scopes → Task 11
   - [x] `argot extract . --scope cli` runs one scope → Task 11
   - [x] `argot check <file>` / `argot check <ref>` auto-routes by prefix → Task 13
   - [x] `argot status` shows per-scope info → Task 14
   - [x] No-config repos behave exactly as before → Tasks 6 (default scope), 11, 13, 14, 16
   - [x] Engine `--path-prefix` filter added to `extract.py` → Task 2
   - [x] CLI passes `--path-prefix` to engine subprocess → Tasks 9, 10, 11, 13
   - [x] Tests alongside new logic → Tasks 1, 3, 5, 15, 16

2. **Placeholder scan:** no "TBD", "similar to", "add validation" placeholders — each step has real code.

3. **Type consistency:**
   - `ResolvedScope` fields (`name`, `pathPrefix`, `datasetPath`, `modelPath`) are used consistently across `scopes.ts`, adapter, commands.
   - Port shapes include `pathPrefix?: string` in both `EngineRunner.runExtract` and `StyleChecker.runCheck`.
   - `ScopeConfig.path` (config input) vs `ResolvedScope.pathPrefix` (internal) — mapped exactly once in `resolveScopes`.

4. **Backwards compat**
   - Default scope uses identical dataset/model paths as today (`.argot/dataset.jsonl`, `.argot/model.pkl`).
   - When `pathPrefix === ''`, the CLI omits `--path-prefix` from engine argv → byte-identical subprocess invocation.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-19-multi-scope-models.md`.

Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks.
2. **Inline Execution** — execute tasks in this session using executing-plans with checkpoints.

Which approach?
