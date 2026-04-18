# explain output redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `argot explain` output feel like a natural continuation of `argot check` — each violation section has an 80-char header mirroring the check row format, all sections render in stable order, and a count line tells the user what's coming.

**Architecture:** Buffer all engine records as they arrive, fire all Claude API calls in parallel immediately, wait for all results on engine close, then render in arrival order. The engine (local inference) finishes in milliseconds; Claude calls dominate latency and run in parallel regardless. One new helper `_score_to_tag` (Python) mirrors check's tag logic exactly. The adapter drops its streaming-render approach in favour of buffer-and-render.

**Tech Stack:** Python (argot engine, argparse, json), TypeScript/Bun (Effect v4 beta, Schema, Node readline), `just verify` for validation.

---

## File map

| File | Change |
|------|--------|
| `engine/argot/explain.py` | Add `_score_to_tag()`, emit `tag` in each JSON record |
| `engine/argot/tests/test_explain.py` | Add `_score_to_tag` unit tests |
| `engine/argot/tests/test_explain_smoke.py` | Assert `tag` field present and valid in smoke output |
| `cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts` | Add `tag` to schema, replace streaming-render with buffer-and-render, add `formatSeparator` |
| `cli/src/shell/infrastructure/adapters/in/commands/explain.command.ts` | Add `threshold` to header line |

---

## Task 1: Add `_score_to_tag` unit tests (failing)

**Files:**
- Modify: `engine/argot/tests/test_explain.py`

- [ ] **Step 1: Add failing tests**

Append to `engine/argot/tests/test_explain.py` (after the existing imports, add `_score_to_tag` to the import line):

```python
from argot.explain import percentile_rank, select_style_examples, _score_to_tag
```

Then add at the bottom of the file:

```python
def test_score_to_tag_unusual() -> None:
    assert _score_to_tag(0.6, 0.5) == "unusual"


def test_score_to_tag_suspicious() -> None:
    assert _score_to_tag(0.9, 0.5) == "suspicious"


def test_score_to_tag_foreign() -> None:
    assert _score_to_tag(1.2, 0.5) == "foreign"


def test_score_to_tag_boundary_unusual() -> None:
    # exactly at threshold + 0.3 → still "unusual"
    assert _score_to_tag(0.8, 0.5) == "unusual"


def test_score_to_tag_boundary_suspicious() -> None:
    # exactly at threshold + 0.6 → still "suspicious"
    assert _score_to_tag(1.1, 0.5) == "suspicious"
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /path/to/argot && uv run pytest engine/argot/tests/test_explain.py -v -k "score_to_tag"
```

Expected: `ImportError: cannot import name '_score_to_tag' from 'argot.explain'`

---

## Task 2: Implement `_score_to_tag` and emit `tag` in engine JSON

**Files:**
- Modify: `engine/argot/explain.py`

- [ ] **Step 1: Add `_score_to_tag` function**

Insert after the `percentile_rank` function (after line 24):

```python
def _score_to_tag(score: float, threshold: float) -> str:
    if score <= threshold + 0.3:
        return "unusual"
    elif score <= threshold + 0.6:
        return "suspicious"
    else:
        return "foreign"
```

- [ ] **Step 2: Add `tag` to the JSON payload**

In `_emit_patches`, locate the `print(json.dumps({...}))` block and add `"tag"` after `"percentile"`:

```python
                    print(
                        json.dumps(
                            {
                                "file_path": file_path,
                                "line": hunk.new_start,
                                "commit": commit_label,
                                "surprise": round(score, 4),
                                "percentile": round(pct, 1),
                                "tag": _score_to_tag(score, args.threshold),
                                "hunk_text": hunk_text,
                                "context_text": ctx_text,
                                "style_examples": example_texts,
                            }
                        )
                    )
```

- [ ] **Step 3: Run the unit tests to verify they pass**

```bash
uv run pytest engine/argot/tests/test_explain.py -v -k "score_to_tag"
```

Expected: 5 tests pass.

- [ ] **Step 4: Run the full test suite**

```bash
just test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/explain.py engine/argot/tests/test_explain.py
git commit -m "feat(engine): add _score_to_tag and emit tag field in explain JSON"
```

---

## Task 3: Assert `tag` in smoke test

**Files:**
- Modify: `engine/argot/tests/test_explain_smoke.py`

- [ ] **Step 1: Add assertion**

In `test_explain_workdir_mode_with_changes_emits_json`, after the existing assertions at the bottom of the function, add:

```python
    assert "tag" in record
    assert record["tag"] in {"unusual", "suspicious", "foreign"}
```

The full assertion block at the end of that function should now read:

```python
    assert record["commit"] == "workdir"
    assert "file_path" in record
    assert "hunk_text" in record
    assert "tag" in record
    assert record["tag"] in {"unusual", "suspicious", "foreign"}
```

- [ ] **Step 2: Run smoke tests**

```bash
uv run pytest engine/argot/tests/test_explain_smoke.py -v
```

Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add engine/argot/tests/test_explain_smoke.py
git commit -m "test(engine): assert tag field in explain smoke output"
```

---

## Task 4: Restructure adapter — buffer-and-render with 80-char headers

**Files:**
- Modify: `cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts`

The current adapter renders violations immediately as Claude responds, producing non-deterministic order. Replace the entire `runExplain` method body with the buffer-and-render approach.

- [ ] **Step 1: Add `tag` to `EngineRecord` schema**

In `bun-explainer.adapter.ts`, locate the `EngineRecord` schema definition and add `tag`:

```typescript
const EngineRecord = Schema.Struct({
  file_path: Schema.String,
  line: Schema.Number,
  commit: Schema.String,
  surprise: Schema.Number,
  percentile: Schema.Number,
  tag: Schema.String,
  hunk_text: Schema.String,
  style_examples: Schema.Array(Schema.String),
});
```

- [ ] **Step 2: Add `formatSeparator` helper**

Add this function before the `BunExplainerLive` export:

```typescript
function formatSeparator(index: number, total: number, record: typeof EngineRecord.Type): string {
  const info = `── [${index}/${total}] ${record.file_path}:${record.line}  ${record.tag}  ${record.surprise.toFixed(4)}  ${record.commit} `;
  const padLen = Math.max(0, 80 - info.length);
  return info + '─'.repeat(padLen);
}
```

- [ ] **Step 3: Replace the `runExplain` method body with buffer-and-render**

Replace everything inside the `Effect.callback(...)` call (the body of `runExplain`) with:

```typescript
      Effect.callback<void, ExplainEngineSpawnFailed | ExplainEngineExitNonZero>((resume) => {
        const { cmd, args } = engineCmd('argot.explain');
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
              '--dataset',
              datasetPath,
              '--threshold',
              String(threshold),
            ],
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

        const items: Array<{
          record: typeof EngineRecord.Type;
          explanationPromise: Promise<typeof Explanation.Type>;
        }> = [];

        const rl = createInterface({ input: proc.stdout! });

        rl.on('line', (line: string) => {
          if (!line.trim()) return;
          let record: typeof EngineRecord.Type;
          try {
            record = Effect.runSync(
              Schema.decodeUnknown(Schema.fromJsonString(EngineRecord))(line),
            );
          } catch {
            return;
          }
          items.push({
            record,
            explanationPromise: Effect.runPromise(callClaude(record, claudeModel)),
          });
        });

        proc.on('close', (code: number | null) => {
          Promise.all(items.map(({ explanationPromise }) => explanationPromise))
            .then((explanations) => {
              stopSpinner();
              if (items.length === 0) {
                console.log('No violations above threshold — nothing to explain.');
              } else {
                console.log(`\n${items.length} violation(s) above threshold — explaining...`);
                items.forEach(({ record }, i) => {
                  const explanation = explanations[i]!;
                  console.log('');
                  console.log(formatSeparator(i + 1, items.length, record));
                  console.log('');
                  console.log(`  ${explanation.summary}`);
                  if (explanation.issues.length > 0) {
                    console.log('');
                    for (const issue of explanation.issues) {
                      console.log(`  • ${issue}`);
                    }
                  }
                  console.log('');
                });
              }
              if (code === 0) {
                resume(Effect.void);
              } else {
                const stderr = Buffer.concat(stderrChunks).toString('utf-8');
                resume(Effect.fail(new ExplainEngineExitNonZero({ exitCode: code ?? -1, stderr })));
              }
            })
            .catch((cause: unknown) => {
              stopSpinner();
              resume(
                Effect.fail(
                  new ExplainEngineExitNonZero({
                    exitCode: code ?? -1,
                    stderr: String(cause),
                  }),
                ),
              );
            });
        });
      }),
```

Note: `Schema.decodeUnknown` is the Effect v4 name for what the current code calls `Schema.decodeUnknownEffect`. If the TypeScript compiler rejects `Schema.decodeUnknown`, use `Schema.decodeUnknownEffect` instead — same function, different name across versions.

- [ ] **Step 4: Run type-check and verify**

```bash
just verify
```

Expected: all checks pass. If you get a type error on `Schema.decodeUnknown`, swap it for `Schema.decodeUnknownEffect` (that's what the rest of the file already uses).

- [ ] **Step 5: Commit**

```bash
git add cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts
git commit -m "feat(cli): buffer-and-render explain output with 80-char violation headers"
```

---

## Task 5: Update explain command header to include threshold

**Files:**
- Modify: `cli/src/shell/infrastructure/adapters/in/commands/explain.command.ts`

- [ ] **Step 1: Update the Console.log line**

Find:

```typescript
      yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot}) · model ${ctx.preferences.model}`);
```

Replace with:

```typescript
      yield* Console.log(
        `argot · ${ctx.name} (${ctx.gitRoot}) · threshold ${ctx.preferences.threshold} · model ${ctx.preferences.model}`,
      );
```

- [ ] **Step 2: Run full verify**

```bash
just verify
```

Expected: all checks pass.

- [ ] **Step 3: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/explain.command.ts
git commit -m "feat(cli): add threshold to explain command header line"
```

---

## Self-review

**Spec coverage check:**

| Spec requirement | Covered by |
|-----------------|-----------|
| `tag` field in engine JSON | Task 2 |
| `tag` unit tests | Task 1 |
| Smoke test asserts `tag` | Task 3 |
| `tag` in `EngineRecord` schema | Task 4 step 1 |
| `formatSeparator` 80-char line | Task 4 step 2 |
| Buffer-and-render with `Promise.all` | Task 4 step 3 |
| Zero-violation message | Task 4 step 3 |
| Count line `N violation(s) above threshold — explaining...` | Task 4 step 3 |
| Header includes threshold | Task 5 |

**Placeholder scan:** No TBDs, no "implement later", all code blocks are complete.

**Type consistency:** `EngineRecord.Type` used in `formatSeparator` signature and `items` array — consistent. `Explanation.Type` used in `explanationPromise` and `explanations[i]!` — consistent.
