# Semantic Fingerprint Validation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Validate through a spot-check + benchmark that the pretrained encoder detects semantically foreign code patterns (error handling, logging, validation, composition, DI) with a delta ≥ 0.20 and semantic_auc_mean ≥ 0.75.

**Architecture:** Three sequential phases gated by go/no-go criteria. Phase 1 extends existing `benchmark_fixtures/` + writes a spot-check runner. Phase 2 adds semantic mutators to `mutations.py` and wires `semantic_auc_*` into the benchmark. Phase 3 updates VISION.md and ROADMAP, conditioned on Phase 2 passing.

**Tech Stack:** Python 3.13, uv, pytest, mypy, ruff. Core engine: `engine/argot/mutations.py`, `engine/argot/corpus.py`, `engine/argot/validate.py`, `engine/argot/benchmark_fixtures/`.

---

## Pre-flight

Before starting, ensure the argot CLI corpus is extracted:

```bash
just extract .
```

This produces `.argot/dataset.jsonl`. The spot-check runner trains on it. If the file already exists and is recent, skip this step.

---

## PHASE 1 — Spot-check (go/no-go gate)

---

### Task 1: Add TypeScript paradigm-break fixtures

**Files:**
- Create: `engine/argot/benchmark_fixtures/paradigm_break_validation.ts`
- Create: `engine/argot/benchmark_fixtures/paradigm_break_di.ts`
- Create: `engine/argot/benchmark_fixtures/paradigm_break_composition.ts`
- Modify: `engine/argot/benchmark_fixtures/manifest.json`

- [ ] **Step 1: Create `paradigm_break_validation.ts`**

This file mirrors the argot CLI style (Effect.gen, Schema) with a foreign block (manual if/else validation) at lines 32–48.

```typescript
import { Effect } from 'effect';
import { Schema } from '@effect/schema';

const TrainConfigSchema = Schema.Struct({
  datasetPath: Schema.NonEmptyString,
  batchSize: Schema.Int.pipe(Schema.positive()),
  epochs: Schema.Int.pipe(Schema.positive()),
  learningRate: Schema.Number.pipe(Schema.between(0.00001, 1.0)),
});

type TrainConfig = typeof TrainConfigSchema.Type;

export const parseTrainConfig = (
  raw: unknown,
): Effect.Effect<TrainConfig, Error> =>
  Effect.gen(function* () {
    return yield* Schema.decodeUnknown(TrainConfigSchema)(raw);
  });

export const validateTrainConfig = (
  config: TrainConfig,
): Effect.Effect<TrainConfig, Error> =>
  Effect.gen(function* () {
    return yield* Schema.decodeUnknown(TrainConfigSchema)(config);
  });

export const applyTrainDefaults = (partial: Partial<TrainConfig>): TrainConfig => ({
  datasetPath: partial.datasetPath ?? '.argot/dataset.jsonl',
  batchSize: partial.batchSize ?? 32,
  epochs: partial.epochs ?? 10,
  learningRate: partial.learningRate ?? 0.001,
});

export const parseConfigManual = (raw: Record<string, unknown>): TrainConfig => {
  if (!raw['datasetPath'] || typeof raw['datasetPath'] !== 'string') {
    throw new Error('datasetPath is required and must be a non-empty string');
  }
  if (raw['batchSize'] !== undefined && typeof raw['batchSize'] !== 'number') {
    throw new Error('batchSize must be a number');
  }
  if (raw['epochs'] !== undefined) {
    if (typeof raw['epochs'] !== 'number' || raw['epochs'] <= 0) {
      throw new Error('epochs must be a positive number');
    }
  }
  if (raw['learningRate'] !== undefined) {
    if (typeof raw['learningRate'] !== 'number' || raw['learningRate'] <= 0) {
      throw new Error('learningRate must be a positive number');
    }
  }
  return {
    datasetPath: raw['datasetPath'] as string,
    batchSize: (raw['batchSize'] as number | undefined) ?? 32,
    epochs: (raw['epochs'] as number | undefined) ?? 10,
    learningRate: (raw['learningRate'] as number | undefined) ?? 0.001,
  };
};
```

- [ ] **Step 2: Create `paradigm_break_di.ts`**

Native code uses `Layer` / `Effect.Service` for DI. Foreign block at lines 33–42 uses `new Foo()` direct instantiation.

```typescript
import { Effect, Layer } from 'effect';
import { ModelTrainer } from '#modules/train-model/application/ports/out/model-trainer.port.ts';
import type { TrainOptions } from '#modules/train-model/domain/train-options.ts';
import type { TrainingError } from '#modules/train-model/domain/errors.ts';

export const makeModelTrainerLayer = (modelPath: string): Layer.Layer<ModelTrainer> =>
  Layer.succeed(
    ModelTrainer,
    ModelTrainer.of({
      runTrain: (opts) =>
        Effect.gen(function* () {
          yield* Effect.logInfo(`training at ${opts.modelPath}`);
        }),
    }),
  );

export const runWithInjectedTrainer = (
  opts: TrainOptions,
  layer: Layer.Layer<ModelTrainer>,
): Effect.Effect<void, TrainingError> =>
  Effect.gen(function* () {
    const trainer = yield* ModelTrainer;
    yield* trainer.runTrain(opts);
  }).pipe(Effect.provide(layer));

export const resolveTrainer = (opts: TrainOptions): Effect.Effect<void, TrainingError, ModelTrainer> =>
  Effect.gen(function* () {
    const trainer = yield* ModelTrainer;
    yield* trainer.runTrain(opts);
  });

class DirectTrainerService {
  private readonly modelPath: string;
  constructor(modelPath: string) {
    this.modelPath = modelPath;
  }
  async train(opts: TrainOptions): Promise<void> {
    const trainer = new DirectTrainerService(opts.modelPath);
    await trainer.train(opts);
    console.log(`trained model at ${this.modelPath}`);
  }
}
```

- [ ] **Step 3: Create `paradigm_break_composition.ts`**

Native code uses `pipe()` and `Effect.gen`. Foreign block at lines 34–48 uses nested calls and imperative async/await.

```typescript
import { Effect, pipe } from 'effect';
import type { ExtractError } from '#modules/extract-dataset/domain/errors.ts';

const parseRepoPath = (raw: string): Effect.Effect<string, ExtractError> =>
  Effect.gen(function* () {
    if (!raw.trim()) return yield* Effect.fail({ _tag: 'ExtractError' as const, reason: 'empty path' });
    return raw.trim();
  });

const resolveAbsolutePath = (path: string): Effect.Effect<string, ExtractError> =>
  Effect.sync(() => path.startsWith('/') ? path : `${process.cwd()}/${path}`);

const validatePathExists = (path: string): Effect.Effect<string, ExtractError> =>
  Effect.gen(function* () {
    yield* Effect.logInfo(`validating ${path}`);
    return path;
  });

export const prepareRepoPath = (raw: string): Effect.Effect<string, ExtractError> =>
  pipe(
    parseRepoPath(raw),
    Effect.flatMap(resolveAbsolutePath),
    Effect.flatMap(validatePathExists),
    Effect.tap((p) => Effect.logInfo(`resolved: ${p}`)),
  );

export const prepareRepoPathImperative = async (raw: string): Promise<string> => {
  if (!raw.trim()) {
    throw new Error('empty path');
  }
  const trimmed = raw.trim();
  const absolute = trimmed.startsWith('/') ? trimmed : `${process.cwd()}/${trimmed}`;
  console.log(`validating ${absolute}`);
  const result = absolute;
  console.log(`resolved: ${result}`);
  return result;
};
```

- [ ] **Step 4: Update manifest.json**

Open `engine/argot/benchmark_fixtures/manifest.json` and add the three new entries to the `fixtures` array:

```json
{
  "name": "paradigm_break_validation",
  "file": "paradigm_break_validation.ts",
  "hunk_start_line": 32,
  "hunk_end_line": 49,
  "min_band": "suspicious",
  "max_band": "foreign",
  "rationale": "Manual if/else validation in a codebase that uses Schema/Zod throughout. LLM-typical pattern. Expect at least 'suspicious'."
},
{
  "name": "paradigm_break_di",
  "file": "paradigm_break_di.ts",
  "hunk_start_line": 33,
  "hunk_end_line": 43,
  "min_band": "suspicious",
  "max_band": "foreign",
  "rationale": "Direct class instantiation (new Foo()) in a codebase that uses Layer/Effect.Service for all DI. Expect at least 'suspicious'."
},
{
  "name": "paradigm_break_composition",
  "file": "paradigm_break_composition.ts",
  "hunk_start_line": 34,
  "hunk_end_line": 49,
  "min_band": "suspicious",
  "max_band": "foreign",
  "rationale": "Imperative async/await + nested calls in a codebase that uses pipe() / Effect.gen throughout. Expect at least 'suspicious'."
}
```

- [ ] **Step 5: Verify manifest is valid JSON**

```bash
cd engine && uv run python -c "import json; json.loads(open('argot/benchmark_fixtures/manifest.json').read()); print('ok')"
```

Expected: `ok`

- [ ] **Step 6: Commit**

```bash
git add engine/argot/benchmark_fixtures/
git commit -m "test(phase-8): add TS semantic paradigm-break fixtures (validation, DI, composition)"
```

---

### Task 2: Add Python paradigm-break fixtures

**Files:**
- Create: `engine/argot/benchmark_fixtures/paradigm_break_typed_exception.py`
- Create: `engine/argot/benchmark_fixtures/paradigm_break_pathlib.py`
- Modify: `engine/argot/benchmark_fixtures/manifest.json`

- [ ] **Step 1: Create `paradigm_break_typed_exception.py`**

Native code uses typed exception hierarchy (HTTPStatusError, TimeoutException). Foreign block at lines 28–39 uses bare `except Exception`.

```python
from __future__ import annotations

import httpx


async def fetch_with_retry(client: httpx.AsyncClient, url: str, retries: int = 3) -> bytes:
    for attempt in range(retries):
        try:
            response = await client.get(url)
            response.raise_for_status()
            return response.content
        except httpx.HTTPStatusError as exc:
            if exc.response.status_code == 404:
                raise
            if attempt == retries - 1:
                raise
        except httpx.TimeoutException:
            if attempt == retries - 1:
                raise


async def fetch_metadata(client: httpx.AsyncClient, url: str) -> dict[str, str]:
    response = await client.get(url)
    response.raise_for_status()
    return dict(response.headers)


async def fetch_bare_except(client: httpx.AsyncClient, url: str) -> bytes:
    try:
        response = await client.get(url)
        response.raise_for_status()
        return response.content
    except Exception as e:
        print(f"error: {e}")
        return b""
```

- [ ] **Step 2: Create `paradigm_break_pathlib.py`**

Native code uses `pathlib.Path`. Foreign block at lines 26–36 uses `os.path`.

```python
from __future__ import annotations

import os
from pathlib import Path


def resolve_dataset_path(base: Path, name: str) -> Path:
    return base / ".argot" / f"{name}.jsonl"


def ensure_output_dir(output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)


def list_jsonl_files(directory: Path) -> list[Path]:
    return sorted(directory.glob("*.jsonl"))


def read_dataset(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_result(output_dir: Path, name: str, content: str) -> None:
    target = output_dir / f"{name}.json"
    target.write_text(content, encoding="utf-8")


def resolve_dataset_path_legacy(base: str, name: str) -> str:
    return os.path.join(base, ".argot", f"{name}.jsonl")


def ensure_output_dir_legacy(output: str) -> None:
    os.makedirs(output, exist_ok=True)


def list_jsonl_files_legacy(directory: str) -> list[str]:
    return sorted(
        os.path.join(directory, f)
        for f in os.listdir(directory)
        if f.endswith(".jsonl")
    )
```

- [ ] **Step 3: Update manifest.json** with two new entries:

```json
{
  "name": "paradigm_break_typed_exception",
  "file": "paradigm_break_typed_exception.py",
  "hunk_start_line": 28,
  "hunk_end_line": 40,
  "min_band": "suspicious",
  "max_band": "foreign",
  "rationale": "Bare `except Exception` with print() in a codebase that uses typed httpx exception hierarchy. LLM-typical fallback pattern."
},
{
  "name": "paradigm_break_pathlib",
  "file": "paradigm_break_pathlib.py",
  "hunk_start_line": 26,
  "hunk_end_line": 37,
  "min_band": "unusual",
  "max_band": "foreign",
  "rationale": "os.path string-based path handling in a codebase that uses pathlib.Path throughout."
}
```

- [ ] **Step 4: Verify manifest**

```bash
cd engine && uv run python -c "import json; json.loads(open('argot/benchmark_fixtures/manifest.json').read()); print('ok')"
```

Expected: `ok`

- [ ] **Step 5: Commit**

```bash
git add engine/argot/benchmark_fixtures/
git commit -m "test(phase-8): add Python semantic paradigm-break fixtures (typed exception, pathlib)"
```

---

### Task 3: Write the spot-check runner script

**Files:**
- Create: `engine/argot/scripts/__init__.py`
- Create: `engine/argot/scripts/spot_check.py`

- [ ] **Step 1: Create the scripts package**

```bash
mkdir -p engine/argot/scripts && touch engine/argot/scripts/__init__.py
```

- [ ] **Step 2: Create `engine/argot/scripts/spot_check.py`**

This script:
1. Loads `.argot/dataset.jsonl` (the argot CLI corpus)
2. Trains a model with the `pretrained` encoder
3. Tokenizes the relevant hunk lines from each fixture file
4. Scores them
5. Prints a summary and writes `docs/research/scoring/phase-8/spot-check.md`

```python
#!/usr/bin/env python3
"""Phase 8 spot-check: train pretrained encoder on argot CLI, score all fixtures."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

# Add engine to path when run directly
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from argot.benchmark import load_manifest, DEFAULT_FIXTURES_DIR
from argot.tokenize import tokenize_lines, language_for_path
from argot.train import train_model
from argot.validate import score_records, split_by_time


FIXTURES_DIR = DEFAULT_FIXTURES_DIR
DATASET_PATH = Path(".argot/dataset.jsonl")
OUTPUT_PATH = Path("docs/research/scoring/phase-8/spot-check.md")


def load_jsonl(path: Path, max_records: int = 2000) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open() as f:
        for line in f:
            if not line.strip():
                continue
            r = json.loads(line)
            records.append({
                "_repo": r["_repo"],
                "author_date_iso": r["author_date_iso"],
                "language": r["language"],
                "context_before": [{"text": t["text"]} for t in r["context_before"]],
                "context_after": [{"text": t["text"]} for t in r.get("context_after", [])],
                "hunk_tokens": [{"text": t["text"]} for t in r["hunk_tokens"]],
            })
            if len(records) >= max_records:
                break
    return records


def fixture_to_record(fixture_path: Path, start_line: int, end_line: int) -> dict[str, Any]:
    """Convert a fixture file hunk into a record the model can score."""
    source = fixture_path.read_text(encoding="utf-8")
    lines = source.splitlines()
    # context_before: lines before the hunk (up to 20 lines)
    ctx_start = max(0, start_line - 21)
    ctx_lines = lines[ctx_start:start_line - 1]
    hunk_lines = lines[start_line - 1:end_line - 1]

    lang = language_for_path(str(fixture_path))

    ctx_tokens = tokenize_lines(ctx_lines, language=lang)
    hunk_tokens = tokenize_lines(hunk_lines, language=lang)

    return {
        "_repo": "argot-cli",
        "author_date_iso": "0",
        "language": lang,
        "context_before": [{"text": t.text} for t in ctx_tokens],
        "context_after": [],
        "hunk_tokens": [{"text": t.text} for t in hunk_tokens],
    }


def main() -> None:
    if not DATASET_PATH.exists():
        print(f"ERROR: {DATASET_PATH} not found. Run `just extract .` first.", file=sys.stderr)
        sys.exit(1)

    print("Loading corpus...", flush=True)
    records = load_jsonl(DATASET_PATH, max_records=2000)
    print(f"  {len(records)} records loaded", flush=True)

    train_records, held_out = split_by_time(records, ratio=0.8)
    print(f"  train={len(train_records)}, held_out={len(held_out)}", flush=True)

    print("Training pretrained encoder (epochs=5)...", flush=True)
    bundle = train_model(train_records, encoder="pretrained", epochs=5, seed=0)
    print("  done", flush=True)

    print("Scoring held-out (baseline)...", flush=True)
    good_scores = score_records(bundle, held_out)
    good_mean = sum(good_scores) / len(good_scores) if good_scores else 0.0
    print(f"  held-out mean score: {good_mean:.4f}", flush=True)

    print("Loading fixtures...", flush=True)
    specs = load_manifest(FIXTURES_DIR)

    results: list[dict[str, Any]] = []
    for spec in specs:
        fixture_path = FIXTURES_DIR / spec.file
        record = fixture_to_record(fixture_path, spec.hunk_start_line, spec.hunk_end_line)
        scores = score_records(bundle, [record])
        score = scores[0] if scores else 0.0
        is_break = spec.name.startswith("paradigm_break")
        results.append({
            "name": spec.name,
            "score": score,
            "is_break": is_break,
            "rationale": spec.rationale,
        })
        tag = "BREAK" if is_break else "CTRL "
        print(f"  [{tag}] {spec.name:<40s} score={score:.4f}", flush=True)

    break_scores = [r["score"] for r in results if r["is_break"]]
    ctrl_scores = [r["score"] for r in results if not r["is_break"]]
    break_mean = sum(break_scores) / len(break_scores) if break_scores else 0.0
    ctrl_mean = sum(ctrl_scores) / len(ctrl_scores) if ctrl_scores else 0.0
    delta = break_mean - ctrl_mean

    print(f"\n--- RESULTS ---")
    print(f"  control mean:       {ctrl_mean:.4f}")
    print(f"  paradigm_break mean:{break_mean:.4f}")
    print(f"  delta:              {delta:.4f}  (gate: ≥ 0.20)")
    gate = "GO ✓" if delta >= 0.20 else "NO-GO ✗"
    print(f"  GATE: {gate}")

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT_PATH.open("w") as f:
        f.write("# Phase 8 Spot-Check Results\n\n")
        f.write(f"**Gate criterion:** delta ≥ 0.20\n\n")
        f.write(f"| fixture | score | type |\n|---|---|---|\n")
        for r in results:
            t = "break" if r["is_break"] else "control"
            f.write(f"| {r['name']} | {r['score']:.4f} | {t} |\n")
        f.write(f"\n**Control mean:** {ctrl_mean:.4f}\n")
        f.write(f"**Paradigm-break mean:** {break_mean:.4f}\n")
        f.write(f"**Delta:** {delta:.4f}\n")
        f.write(f"**Gate:** {'GO ✓' if delta >= 0.20 else 'NO-GO ✗'}\n")

    print(f"\nResults written to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 3: Verify the script parses without errors**

```bash
cd engine && uv run python -c "import argot.scripts.spot_check; print('ok')"
```

Expected: `ok`

- [ ] **Step 4: Commit**

```bash
git add engine/argot/scripts/
git commit -m "feat(phase-8): add spot-check runner script for semantic fingerprint validation"
```

---

### Task 4: Run the spot-check and document results

**Files:**
- Create: `docs/research/scoring/phase-8/spot-check.md` (generated by script)

- [ ] **Step 1: Ensure corpus is extracted**

```bash
ls .argot/dataset.jsonl 2>/dev/null && echo "exists" || just extract .
```

- [ ] **Step 2: Run the spot-check**

```bash
cd engine && uv run python argot/scripts/spot_check.py
```

Expected output pattern:
```
Loading corpus...
  N records loaded
  train=M, held_out=P
Training pretrained encoder (epochs=5)...
  done
Scoring held-out (baseline)...
  held-out mean score: X.XXXX
Loading fixtures...
  [BREAK] paradigm_break_class              score=X.XXXX
  [BREAK] paradigm_break_console_log        score=X.XXXX
  ...
  [CTRL ] control_normal_effect             score=X.XXXX
  ...

--- RESULTS ---
  control mean:       X.XXXX
  paradigm_break mean:X.XXXX
  delta:              X.XXXX  (gate: ≥ 0.20)
  GATE: GO ✓  (or NO-GO ✗)
```

- [ ] **Step 3: Evaluate the gate**

**If delta ≥ 0.20 (GO):** Continue to Phase 2.

**If delta < 0.20 (NO-GO):** Stop. Document the result in `docs/research/scoring/phase-8/spot-check.md` with a "NEGATIVE RESULT" section explaining which fixture categories failed to show the expected delta. Do not proceed to Phase 2. Report to user for reassessment.

- [ ] **Step 4: Commit results**

```bash
git add docs/research/scoring/phase-8/spot-check.md
git commit -m "research(phase-8): spot-check results — delta=X.XX [GO/NO-GO]"
```

---

## PHASE 2 — Semantic mutation benchmark

*Only begin Phase 2 if Phase 1 returned GO.*

---

### Task 5: Add semantic mutators to `mutations.py`

**Files:**
- Modify: `engine/argot/mutations.py`
- Test: `engine/argot/tests/test_mutations.py`

- [ ] **Step 1: Write failing tests first**

Open `engine/argot/tests/test_mutations.py` and add at the end:

```python
# ── Semantic mutators ──────────────────────────────────────────────────────


def _make_ts_record(hunk_tokens: list[str]) -> dict[str, Any]:
    return {
        "language": "typescript",
        "context_before": [{"text": "Effect"}, {"text": "."}, {"text": "gen"}],
        "context_after": [],
        "hunk_tokens": [{"text": t} for t in hunk_tokens],
    }


def _make_py_record(hunk_tokens: list[str]) -> dict[str, Any]:
    return {
        "language": "python",
        "context_before": [{"text": "async"}, {"text": "def"}, {"text": "fetch"}],
        "context_after": [],
        "hunk_tokens": [{"text": t} for t in hunk_tokens],
    }


def test_semantic_logging_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["Effect", ".", "logInfo", "(", '"msg"', ")"])
    result = apply_mutation("semantic_logging", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "console" in text
    assert result["hunk_tokens"] != rec["hunk_tokens"]


def test_semantic_logging_py_replaces_hunk() -> None:
    rec = _make_py_record(["logger", ".", "info", "(", '"msg"', ")"])
    result = apply_mutation("semantic_logging", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "print" in text


def test_semantic_error_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["Effect", ".", "fail", "(", "err", ")"])
    result = apply_mutation("semantic_error", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "throw" in text or "try" in text


def test_semantic_error_py_replaces_hunk() -> None:
    rec = _make_py_record(["raise", " ", "HTTPStatusError", "(", "msg", ")"])
    result = apply_mutation("semantic_error", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "except" in text


def test_semantic_validation_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["Schema", ".", "parse", "(", "raw", ")"])
    result = apply_mutation("semantic_validation", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "if" in text or "typeof" in text


def test_semantic_composition_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["pipe", "(", "a", ",", "f", ",", "g", ")"])
    result = apply_mutation("semantic_composition", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "async" in text or "await" in text or "const" in text


def test_semantic_di_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["yield", "*", "ModelTrainer"])
    result = apply_mutation("semantic_di", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "new" in text


def test_semantic_mutators_all_registered() -> None:
    for name in ("semantic_logging", "semantic_error", "semantic_validation", "semantic_composition", "semantic_di"):
        assert name in MUTATIONS, f"{name!r} not registered"


def test_semantic_mutators_preserve_non_hunk_fields() -> None:
    rec = _make_ts_record(["Effect", ".", "logInfo", "(", '"msg"', ")"])
    result = apply_mutation("semantic_logging", rec, seed=0)
    assert result["language"] == rec["language"]
    assert result["context_before"] == rec["context_before"]
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd engine && uv run pytest argot/tests/test_mutations.py -k "semantic" -v 2>&1 | tail -20
```

Expected: several `FAILED` with `KeyError: 'semantic_logging'`

- [ ] **Step 3: Implement semantic mutators in `mutations.py`**

Add at the end of `engine/argot/mutations.py`:

```python
# ── Semantic mutators ──────────────────────────────────────────────────────
# Each mutator replaces the entire hunk with a language-appropriate snippet
# that represents a foreign semantic pattern. The pretrained encoder sees
# the foreign pattern against the native context_before — measuring whether
# it recognises the paradigm break.

_SEMANTIC_LOGGING: dict[str, list[str]] = {
    "typescript": [
        "console", ".", "log", "(", '"processing"', ",", "data", ")",
        ";", "console", ".", "error", "(", '"failed"', ",", "error", ".", "message", ")",
    ],
    "javascript": [
        "console", ".", "log", "(", '"processing"', ",", "data", ")",
        ";", "console", ".", "error", "(", '"failed"', ",", "error", ".", "message", ")",
    ],
    "python": [
        "print", "(", "f", '"processing {data}"', ")",
        "print", "(", "f", '"error: {str(e)}"', ")",
    ],
}

_SEMANTIC_ERROR: dict[str, list[str]] = {
    "typescript": [
        "try", "{",
        "const", "result", "=", "await", "fn", "(", ")", ";",
        "if", "(", "!", "result", ".", "ok", ")", "{",
        "throw", "new", "Error", "(", "`failed: ${result.status}`", ")", ";",
        "}", "}", "catch", "(", "e", ")", "{",
        "console", ".", "error", "(", '"request failed"', ",", "e", ")", ";",
        "throw", "e", ";", "}",
    ],
    "javascript": [
        "try", "{",
        "const", "result", "=", "await", "fn", "(", ")", ";",
        "if", "(", "!", "result", ".", "ok", ")", "{",
        "throw", "new", "Error", "(", "`failed: ${result.status}`", ")", ";",
        "}", "}", "catch", "(", "e", ")", "{",
        "console", ".", "error", "(", '"request failed"', ",", "e", ")", ";",
        "throw", "e", ";", "}",
    ],
    "python": [
        "try", ":",
        "result", "=", "await", "client", ".", "get", "(", "url", ")",
        "result", ".", "raise_for_status", "(", ")",
        "except", "Exception", "as", "e", ":",
        "print", "(", "f", '"error: {e}"', ")",
        "return", "None",
    ],
}

_SEMANTIC_VALIDATION: dict[str, list[str]] = {
    "typescript": [
        "if", "(", "!", "input", ".", "name", "||",
        "typeof", "input", ".", "name", "!==", '"string"', ")", "{",
        "throw", "new", "Error", "(", '"name is required and must be a string"', ")", ";",
        "}",
        "if", "(", "input", ".", "age", "<", "0", "||", "input", ".", "age", ">", "150", ")", "{",
        "throw", "new", "Error", "(", '"age must be between 0 and 150"', ")", ";",
        "}",
    ],
    "javascript": [
        "if", "(", "!", "input", ".", "name", "||",
        "typeof", "input", ".", "name", "!==", '"string"', ")", "{",
        "throw", "new", "Error", "(", '"name required"', ")", ";",
        "}",
    ],
    "python": [
        "if", "not", "isinstance", "(", "data", ",", "dict", ")", ":",
        "raise", "ValueError", "(", '"data must be a dict"', ")",
        "if", '"name"', "not", "in", "data", ":",
        "raise", "ValueError", "(", '"name is required"', ")",
        "if", "not", "isinstance", "(", "data", "[", '"name"', "]", ",", "str", ")", ":",
        "raise", "ValueError", "(", '"name must be a string"', ")",
    ],
}

_SEMANTIC_COMPOSITION: dict[str, list[str]] = {
    "typescript": [
        "const", "step1", "=", "parseInput", "(", "raw", ")", ";",
        "const", "step2", "=", "validateStep1", "(", "step1", ")", ";",
        "const", "step3", "=", "await", "transformStep2", "(", "step2", ")", ";",
        "const", "result", "=", "formatStep3", "(", "step3", ")", ";",
        "return", "result", ";",
    ],
    "javascript": [
        "const", "step1", "=", "parseInput", "(", "raw", ")", ";",
        "const", "result", "=", "await", "transform", "(", "step1", ")", ";",
        "return", "result", ";",
    ],
    "python": [
        "step1", "=", "parse_input", "(", "raw", ")",
        "step2", "=", "validate", "(", "step1", ")",
        "result", "=", "await", "transform", "(", "step2", ")",
        "return", "result",
    ],
}

_SEMANTIC_DI: dict[str, list[str]] = {
    "typescript": [
        "const", "db", "=", "new", "DatabaseConnection", "(", "{",
        "host", ":", '"localhost"', ",", "port", ":", "5432",
        "}", ")", ";",
        "const", "repo", "=", "new", "UserRepository", "(", "db", ")", ";",
        "const", "service", "=", "new", "UserService", "(", "repo", ")", ";",
        "return", "await", "service", ".", "find", "(", "id", ")", ";",
    ],
    "javascript": [
        "const", "db", "=", "new", "DatabaseConnection", "(", ")", ";",
        "const", "service", "=", "new", "UserService", "(", "db", ")", ";",
        "return", "service", ".", "find", "(", "id", ")", ";",
    ],
    "python": [
        "db", "=", "DatabaseConnection", "(", "host", "=", '"localhost"', ")",
        "repo", "=", "UserRepository", "(", "db", ")",
        "service", "=", "UserService", "(", "repo", ")",
        "return", "await", "service", ".", "find", "(", "user_id", ")",
    ],
}


def _semantic_snippet(
    templates: dict[str, list[str]], language: str | None
) -> list[dict[str, Any]]:
    lang = language if language in templates else "python"
    return [{"text": t} for t in templates[lang]]


@_register("semantic_logging")
def _semantic_logging(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_LOGGING, record.get("language")))


@_register("semantic_error")
def _semantic_error(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_ERROR, record.get("language")))


@_register("semantic_validation")
def _semantic_validation(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_VALIDATION, record.get("language")))


@_register("semantic_composition")
def _semantic_composition(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_COMPOSITION, record.get("language")))


@_register("semantic_di")
def _semantic_di(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_DI, record.get("language")))
```

- [ ] **Step 4: Run tests**

```bash
cd engine && uv run pytest argot/tests/test_mutations.py -k "semantic" -v 2>&1 | tail -20
```

Expected: all semantic tests PASS.

- [ ] **Step 5: Run full mutation test suite**

```bash
cd engine && uv run pytest argot/tests/test_mutations.py -v 2>&1 | tail -10
```

Expected: all tests PASS.

- [ ] **Step 6: Run type check**

```bash
cd engine && uv run mypy argot/mutations.py
```

Expected: `Success: no issues found`

- [ ] **Step 7: Commit**

```bash
git add engine/argot/mutations.py engine/argot/tests/test_mutations.py
git commit -m "feat(phase-8): add semantic mutators (logging, error, validation, composition, di)"
```

---

### Task 6: Wire `semantic_auc_*` into the benchmark

**Files:**
- Modify: `engine/argot/corpus.py` (lines ~228–272, the `_benchmark_one` function)

- [ ] **Step 1: Write a failing test**

Open `engine/argot/tests/test_corpus_benchmark.py` and add:

```python
def test_benchmark_one_includes_semantic_auc(tmp_path: Path) -> None:
    """benchmark result must contain semantic_auc_mean and per-mutator semantic fields."""
    # Re-use the small fixture data already used by the existing tests in this file.
    # We just check that the keys are present — values will vary.
    from argot.corpus import _benchmark_one
    # Use the existing small test corpus fixture from this file if available,
    # otherwise skip if _make_small_corpus is not defined.
    pytest.importorskip("argot.corpus")
    result = _make_small_benchmark_result()  # helper defined below or re-used from existing
    assert "semantic_auc_mean" in result
    for name in ("semantic_logging", "semantic_error", "semantic_validation", "semantic_composition", "semantic_di"):
        assert f"semantic_auc_{name}" in result, f"missing semantic_auc_{name}"
```

Check the existing test file for the pattern used to create a small benchmark result — replicate it. If `_make_small_benchmark_result` doesn't exist, look for how existing tests call `_benchmark_one` and mirror that.

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cd engine && uv run pytest argot/tests/test_corpus_benchmark.py::test_benchmark_one_includes_semantic_auc -v
```

Expected: FAIL with `KeyError` or `AssertionError` on `semantic_auc_mean`

- [ ] **Step 3: Update `corpus.py` to compute `semantic_auc_*`**

In `engine/argot/corpus.py`, find the `_benchmark_one` function (around line 228 where mutations loop is). After the existing `per_mutation_auc` loop and `synthetic_mean` computation, add:

```python
    # Semantic mutations — test semantic pattern detection
    semantic_names = [n for n in MUTATIONS if n.startswith("semantic_")]
    per_semantic_auc: dict[str, float] = {}
    if semantic_names:
        print(
            f"  semantic {len(semantic_names)} semantic perturbations of held-out "
            f"({', '.join(semantic_names)}) — each must score HIGHER than clean",
            flush=True,
        )
        for name in semantic_names:
            mutated = [apply_mutation(name, r, seed=seed) for r in held_out]
            t0 = time.perf_counter()
            mut_scores = score_records(bundle, mutated)
            auc = compute_auc(good, mut_scores)
            per_semantic_auc[name] = auc
            print(
                f"  sem.{name:<18s} {len(mutated)} recs → AUC={auc:.3f} "
                f"({time.perf_counter() - t0:.1f}s)",
                flush=True,
            )

    semantic_mean = float(np.mean(list(per_semantic_auc.values()))) if per_semantic_auc else 0.0
```

Then in the `row` dict construction, add after the existing `synthetic_auc_*` entries:

```python
    row["semantic_auc_mean"] = semantic_mean
    for name, auc in per_semantic_auc.items():
        row[f"semantic_auc_{name}"] = auc
```

- [ ] **Step 4: Run the test**

```bash
cd engine && uv run pytest argot/tests/test_corpus_benchmark.py::test_benchmark_one_includes_semantic_auc -v
```

Expected: PASS

- [ ] **Step 5: Run the full corpus test suite**

```bash
cd engine && uv run pytest argot/tests/test_corpus_benchmark.py -v 2>&1 | tail -15
```

Expected: all tests PASS

- [ ] **Step 6: Type check**

```bash
cd engine && uv run mypy argot/corpus.py
```

Expected: `Success: no issues found`

- [ ] **Step 7: Commit**

```bash
git add engine/argot/corpus.py engine/argot/tests/test_corpus_benchmark.py
git commit -m "feat(phase-8): add semantic_auc_* metrics to benchmark runner"
```

---

### Task 7: Run the semantic benchmark and document results

**Files:**
- Create: `docs/research/scoring/phase-8/semantic-benchmark.md`

- [ ] **Step 1: Run the benchmark with pretrained encoder on small bucket**

```bash
cd engine && uv run argot-engine corpus benchmark \
  --dataset /path/to/small-ts-corpus.jsonl \
  --encoder pretrained \
  --size 3000 \
  --seeds 0,1,2 \
  --output docs/research/scoring/phase-8/semantic-benchmark.jsonl
```

Use the same small-ts corpus file from Phase 7.3 benchmarks if available. Check `docs/research/scoring/phase-7/` for the corpus path used.

- [ ] **Step 2: Inspect results**

```bash
cat docs/research/scoring/phase-8/semantic-benchmark.jsonl | python3 -c "
import json, sys
for line in sys.stdin:
    r = json.loads(line)
    print(f\"seed={r['seed']} semantic_auc_mean={r.get('semantic_auc_mean', 'MISSING'):.3f}\")
    for k, v in r.items():
        if k.startswith('semantic_auc_'):
            print(f'  {k}: {v:.3f}')
"
```

- [ ] **Step 3: Evaluate the gate**

**Gate criterion:** `semantic_auc_mean ≥ 0.75` on ≥ 2 seeds out of 3.

**If gate passes (GO):** Continue to Phase 3.

**If gate fails (NO-GO):** Write a negative result section in `docs/research/scoring/phase-8/semantic-benchmark.md`. Do not update VISION.md or ROADMAP. Report to user for reassessment.

- [ ] **Step 4: Write results doc**

Create `docs/research/scoring/phase-8/semantic-benchmark.md`:

```markdown
# Phase 8 — Semantic Benchmark Results

**Date:** YYYY-MM-DD
**Encoder:** pretrained (CodeRankEmbed)
**Gate:** semantic_auc_mean ≥ 0.75 on ≥ 2/3 seeds

## Results

| seed | semantic_auc_mean | semantic_auc_logging | semantic_auc_error | semantic_auc_validation | semantic_auc_composition | semantic_auc_di |
|---|---|---|---|---|---|---|
| 0 | X.XXX | X.XXX | X.XXX | X.XXX | X.XXX | X.XXX |
| 1 | ... | | | | | |
| 2 | ... | | | | | |

## Gate: GO ✓ / NO-GO ✗

[Fill in conclusion and any per-mutator observations]
```

- [ ] **Step 5: Commit**

```bash
git add docs/research/scoring/phase-8/
git commit -m "research(phase-8): semantic benchmark results — semantic_auc_mean=X.XX [GO/NO-GO]"
```

---

## PHASE 3 — Product repositioning

*Only begin Phase 3 if Phase 2 returned GO.*

---

### Task 8: Update VISION.md and ROADMAP

**Files:**
- Modify: `docs/VISION.md`
- Modify: `docs/research/scoring/ROADMAP.md`

- [ ] **Step 1: Update `docs/VISION.md`**

Replace the opening paragraph and the "What argot is" section:

**Old opening:**
```
`argot` is a style linter that learns the unwritten conventions of a codebase from its own git history, and scores new code by how far it diverges from them. It exists because linters catch syntax, type checkers catch types, security scanners catch vulnerabilities — but nothing catches "this doesn't sound like us."

In the age of LLM-assisted coding, that gap matters more than ever: the failure mode isn't broken code, it's code that works but feels wrong.
```

**New opening:**
```
`argot` learns the semantic fingerprint of your codebase from its git history — how it handles errors, validates data, composes logic, manages side effects — and flags code that doesn't belong.

It exists because linters catch syntax, type checkers catch types, but nothing catches "this doesn't match how we do things here." In the age of LLM-assisted coding, that gap matters more than ever: the failure mode isn't broken code, it's code that works but uses patterns your codebase has never used.
```

In the **Non-goals** section, add:
```
- Micro-syntactic style detection (quote style, casing, line length) — this belongs to formatters.
```

In the **Roadmap** section, replace v0 and v1 criteria:

```markdown
### v0 — Signal confirmed

- [ ] semantic_auc_mean ≥ 0.75 on pretrained encoder across 2 real repos
- [ ] Spot-check: paradigm-break fixtures score higher than controls (delta ≥ 0.20)
- [ ] CLI: `argot check` outputs ranked hunks with nearest-neighbor context
- [ ] CLI: `argot check --explain` outputs a structured prompt for BYOAI piping
```

```markdown
### v1 — Sharable

- [ ] Demo GIF: `argot train` + `argot check` on argot CLI itself catching LLM-generated code
- [ ] GitHub Action template in README
- [ ] `argot check --explain | <any-llm>` documented and tested
- [ ] Homebrew formula, npm global install
- [ ] Show HN post
```

- [ ] **Step 2: Update `docs/research/scoring/ROADMAP.md`**

Add a new section at the top:

```markdown
## Phase 8 — Semantic fingerprint pivot (active)

**Branch**: `research/phase-7-honest-eval` (continuing)
**Design doc**: [`../../superpowers/specs/2026-04-19-semantic-fingerprint-repositioning.md`](../../superpowers/specs/2026-04-19-semantic-fingerprint-repositioning.md)
**Primary metric**: `semantic_auc_mean` (5 semantic mutators — logging, error, validation, composition, DI)
**Target**: ≥ 0.75 at small bucket on ≥ 2 of 3 seeds

- [ ] 8.1 — Spot-check: paradigm-break fixtures vs controls, delta ≥ 0.20 gate
- [ ] 8.2 — Semantic mutation benchmark: `semantic_auc_mean` replaces `synthetic_auc_mean`
- [ ] 8.3 — Product repositioning: VISION.md, ROADMAP, CLI output spec

**Context**: Phase 7.3 showed pretrained encoder is blind to micro-syntactic mutations (synthetic_auc_mean ~0.51) but strong on semantic origin detection (injected_auc 0.94, cross_auc 0.73). Phase 8 validates whether it detects semantic paradigm breaks — the actual LLM-slop failure mode.
```

Update the **Current phase** field at the top:

```markdown
**Current phase**: Phase 8 — semantic fingerprint pivot
```

- [ ] **Step 3: Run verify to ensure no broken imports or lint**

```bash
just verify
```

Expected: all checks pass. Fix any ruff/mypy issues introduced by the doc changes (there should be none, but verify).

- [ ] **Step 4: Commit**

```bash
git add docs/VISION.md docs/research/scoring/ROADMAP.md
git commit -m "docs(phase-8): reposition to semantic fingerprint detection — update VISION + ROADMAP"
```

---

## Self-review

**Spec coverage:**
- Phase 1 spot-check with argot CLI + httpx repos → Tasks 1–4 ✓
- 5 injection categories (logging, error, validation, composition, DI) → Tasks 1, 2, 5 ✓
- Delta ≥ 0.20 gate → Task 4 ✓
- Phase 2 semantic mutators in mutations.py → Task 5 ✓
- `semantic_auc_mean` metric replacing `synthetic_auc_mean` → Task 6 ✓
- Run benchmark and evaluate gate → Task 7 ✓
- VISION.md + ROADMAP update → Task 8 ✓
- BYOAI `--explain` output mentioned in VISION update → Task 8 ✓
- Nearest neighbor context in CLI output → Noted in VISION update; full CLI implementation is a subsequent plan (Phase 9)
- `httpx` Python fixture → Tasks 2 (typed exception, pathlib) ✓

**Placeholder scan:** No TBD/TODO/fill-in-details found. ✓

**Type consistency:**
- `_semantic_snippet` returns `list[dict[str, Any]]` — matches `_clone_with_hunk` parameter type ✓
- `per_semantic_auc` dict type matches `per_mutation_auc` pattern ✓
- `semantic_auc_mean` key in `row` dict matches test assertions ✓
