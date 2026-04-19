# Acceptance Catalog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the old fixture-based benchmark and spot-check scripts with a clean acceptance catalog — one entry per benchmark repo, trained on that repo's own frozen corpus, tested against hand-crafted paradigm-break fixtures derived from deep codebase analysis.

**Architecture:** `engine/argot/acceptance/` holds the runner and catalog. Each catalog entry is a directory containing a frozen `corpus.jsonl`, `scopes.json` (paradigm description per scope), `manifest.json` (fixture routing), and fixture files. The runner trains one pretrained-encoder model per scope, scores fixtures, and gates on per-scope `break_mean − ctrl_mean ≥ 0.20`.

**Tech Stack:** Python 3.13, uv, pytest. Uses existing `argot.train.train_model`, `argot.validate.score_records`, `argot.validate.split_by_time`, `argot.tokenize.tokenize_lines / language_for_path`.

---

## File Map

**Create:**
- `engine/argot/acceptance/__init__.py` — empty package marker
- `engine/argot/acceptance/runner.py` — full runner: load catalog entry, train scope models, score fixtures, gate, write markdown
- `engine/argot/acceptance/catalog/ky/corpus.jsonl` — ~600 frozen records from ky git history
- `engine/argot/acceptance/catalog/ky/scopes.json` — single default scope + paradigm description from analysis
- `engine/argot/acceptance/catalog/ky/manifest.json` — fixture routing for ky
- `engine/argot/acceptance/catalog/ky/fixtures/default/*.ts` — 6–10 fixture files (4–6 breaks, 2–4 controls)
- `engine/argot/tests/test_acceptance_runner.py` — unit tests for runner logic

**Delete:**
- `engine/argot/benchmark_fixtures/` (entire directory)
- `engine/argot/benchmark.py`
- `engine/argot/scripts/spot_check.py`
- `engine/argot/tests/test_benchmark.py`
- `engine/argot/tests/test_density_heads.py`
- `engine/argot/jepa/density_heads.py`

**Modify:**
- `engine/argot/corpus.py` — remove `run_benchmark_density`, `_benchmark_one_density`, `_cmd_benchmark_density`, `benchmark-density` subparser (lines 319–430, 456–470, 522–544); update `benchmark` subcommand help to clarify it is for encoder research
- `engine/argot/train.py` — remove `DensityBundle` dataclass (lines 43–47), `train_bpe_density` function (lines 298–321), import of `density_heads`  (line 19)
- `engine/argot/validate.py` — remove `score_records_density` function (lines 168–174), remove `DensityBundle` from import on line 18
- `justfile` — remove `research-honest-benchmark-density` recipe block

---

## Task 1: Phase 0 — Delete dead code

**Files:**
- Delete: `engine/argot/benchmark_fixtures/`, `engine/argot/benchmark.py`, `engine/argot/scripts/spot_check.py`, `engine/argot/tests/test_benchmark.py`, `engine/argot/tests/test_density_heads.py`, `engine/argot/jepa/density_heads.py`
- Modify: `engine/argot/corpus.py`, `engine/argot/train.py`, `engine/argot/validate.py`, `justfile`

- [ ] **Step 1: Delete the dead files**

```bash
rm -rf engine/argot/benchmark_fixtures
rm engine/argot/benchmark.py
rm engine/argot/scripts/spot_check.py
rm engine/argot/tests/test_benchmark.py
rm engine/argot/tests/test_density_heads.py
rm engine/argot/jepa/density_heads.py
```

- [ ] **Step 2: Remove density functions from `engine/argot/corpus.py`**

Remove lines 319–430 (`run_benchmark_density` and `_benchmark_one_density`) and lines 456–470 (`_cmd_benchmark_density`).

Remove the `benchmark-density` subparser block from `main()` — the block starting at line 522:
```python
    bench_density_p = sub.add_parser(
        "benchmark-density", help="Run density-head benchmark (BPE encoder + kNN/GMM head)"
    )
    ...
    bench_density_p.set_defaults(func=_cmd_benchmark_density)
```

Update the `benchmark` subparser help string from `"Run AUC benchmark across sizes × seeds"` to `"Encoder research: run AUC benchmark across sizes × seeds (not acceptance testing)"`.

Also remove unused imports introduced by density: `DensityHeadKind`, `train_bpe_density`, `score_records_density` from the import block at the top of corpus.py.

After editing, verify the imports at the top of corpus.py are clean:

```bash
cd engine && uv run python -c "from argot.corpus import run_benchmark, concat_datasets; print('ok')"
```

Expected: `ok`

- [ ] **Step 3: Remove density from `engine/argot/train.py`**

Remove line 19: `from argot.jepa.density_heads import DensityHead, DensityHeadKind, make_head`

Remove the `DensityBundle` dataclass (lines 43–47):
```python
@dataclass
class DensityBundle:
    bpe_vocab: BpeVocab
    encoder: TokenEncoder
    head: DensityHead
    head_kind: DensityHeadKind
```

Remove `train_bpe_density` function (lines 298–321):
```python
def train_bpe_density(
    records: list[dict[str, Any]],
    *,
    epochs: int,
    batch_size: int,
    lr: float,
    lambd: float,
    head_kind: DensityHeadKind,
    seed: int = 0,
) -> DensityBundle:
    ...
```

Verify:
```bash
cd engine && uv run python -c "from argot.train import train_model, ModelBundle; print('ok')"
```

Expected: `ok`

- [ ] **Step 4: Remove density from `engine/argot/validate.py`**

On line 18, remove `DensityBundle` from the import:
```python
# before
from argot.train import _SEQ_LEN, DensityBundle, ModelBundle, _encode_records
# after
from argot.train import _SEQ_LEN, ModelBundle, _encode_records
```

Remove `score_records_density` function (lines 168–174):
```python
def score_records_density(bundle: DensityBundle, records: list[dict[str, Any]]) -> list[float]:
    """Score records using a density head on BPE embeddings (higher = more anomalous)."""
    _, hunk_x = _encode_records(records, bundle.bpe_vocab, _SEQ_LEN)
    bundle.encoder.eval()
    with torch.no_grad():
        emb = bundle.encoder(hunk_x).numpy()
    return bundle.head.score(emb).tolist()  # type: ignore[no-any-return]
```

Verify:
```bash
cd engine && uv run python -c "from argot.validate import score_records, compute_auc; print('ok')"
```

Expected: `ok`

- [ ] **Step 5: Remove density recipes from `justfile`**

Find and remove the `research-honest-benchmark-density` recipe block (all lines referencing `benchmark-density`). The recipe typically looks like:

```
research-honest-benchmark-density head="knn-20" seeds="3" out="...":
    uv run --package argot-engine python -m argot.corpus benchmark-density \
        ...
```

Remove the entire block (all related lines, including continuation lines).

- [ ] **Step 6: Run full test suite to verify nothing is broken**

```bash
just test 2>&1 | tail -30
```

Expected: all tests pass (test_density_heads.py and test_benchmark.py are gone; remaining tests should be green). If `test_corpus_benchmark.py` or `test_mutations.py` reference density, fix those imports too.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: remove dead density/benchmark code and fixtures (Phase 0 cleanup)"
```

---

## Task 2: Acceptance runner framework

**Files:**
- Create: `engine/argot/acceptance/__init__.py`
- Create: `engine/argot/acceptance/runner.py`
- Create: `engine/argot/tests/test_acceptance_runner.py`

- [ ] **Step 1: Create the package marker**

```bash
mkdir -p engine/argot/acceptance/catalog
touch engine/argot/acceptance/__init__.py
```

- [ ] **Step 2: Write the failing tests first**

Create `engine/argot/tests/test_acceptance_runner.py`:

```python
from __future__ import annotations

import json
from pathlib import Path

import pytest

from argot.acceptance.runner import (
    EntryResult,
    ScopeResult,
    fixture_to_record,
    load_corpus,
    load_manifest,
    load_scopes,
    run_entry,
)


def _write_corpus(path: Path, n: int = 40) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    base_ts = 1_700_000_000
    with path.open("w") as f:
        for i in range(n):
            f.write(
                json.dumps({
                    "_repo": "ky",
                    "author_date_iso": str(base_ts + i * 3600),
                    "file_path": f"source/file_{i % 3}.ts",
                    "language": "typescript",
                    "context_before": [{"text": f"ctx_{i % 7}"}],
                    "context_after": [],
                    "hunk_tokens": [{"text": f"hunk_{i % 11}"}],
                })
                + "\n"
            )


def _make_entry(tmp_path: Path) -> Path:
    entry = tmp_path / "ky"
    entry.mkdir()

    (entry / "scopes.json").write_text(
        json.dumps({
            "scopes": [
                {"name": "default", "path_prefix": "", "paradigm": "Fetch-based HTTP"}
            ]
        })
    )

    fixture_dir = entry / "fixtures" / "default"
    fixture_dir.mkdir(parents=True)
    (fixture_dir / "break.ts").write_text(
        "// fixture\nconst x = 1;\nconst y = 2;\nconst z = x + y;\n"
    )
    (fixture_dir / "control.ts").write_text(
        "// control\nconst a = 1;\nconst b = 2;\nconst c = a + b;\n"
    )

    (entry / "manifest.json").write_text(
        json.dumps({
            "fixtures": [
                {
                    "name": "break_example",
                    "scope": "default",
                    "file": "fixtures/default/break.ts",
                    "hunk_start_line": 2,
                    "hunk_end_line": 4,
                    "is_break": True,
                    "rationale": "Test break fixture",
                },
                {
                    "name": "control_example",
                    "scope": "default",
                    "file": "fixtures/default/control.ts",
                    "hunk_start_line": 2,
                    "hunk_end_line": 4,
                    "is_break": False,
                    "rationale": "Test control fixture",
                },
            ]
        })
    )

    _write_corpus(entry / "corpus.jsonl", n=40)
    return entry


def test_load_scopes(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    scopes = load_scopes(entry)
    assert len(scopes) == 1
    assert scopes[0].name == "default"
    assert scopes[0].path_prefix == ""
    assert scopes[0].paradigm == "Fetch-based HTTP"


def test_load_manifest(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    specs = load_manifest(entry)
    assert len(specs) == 2
    assert specs[0].name == "break_example"
    assert specs[0].is_break is True
    assert specs[1].is_break is False


def test_load_corpus(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    records = load_corpus(entry)
    assert len(records) == 40
    assert all("hunk_tokens" in r for r in records)


def test_fixture_to_record_returns_tokens(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    specs = load_manifest(entry)
    record = fixture_to_record(entry, specs[0])
    assert record["language"] == "typescript"
    assert len(record["hunk_tokens"]) > 0
    assert all("text" in t for t in record["hunk_tokens"])


def test_run_entry_returns_entry_result(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    result = run_entry(entry, epochs=5)
    assert isinstance(result, EntryResult)
    assert result.entry == "ky"
    assert len(result.scope_results) == 1
    assert isinstance(result.scope_results[0], ScopeResult)
    assert result.scope_results[0].name == "default"
    assert len(result.fixture_scores) == 2


def test_run_entry_passed_reflects_gate(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    result = run_entry(entry, epochs=5)
    # passed must equal whether all scopes clear the gate
    expected = all(s.passed for s in result.scope_results)
    assert result.passed == expected


def test_run_entry_raises_on_too_few_records(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    _write_corpus(entry / "corpus.jsonl", n=5)
    with pytest.raises(RuntimeError, match="only.*records"):
        run_entry(entry, epochs=5)
```

- [ ] **Step 3: Run tests — verify they fail**

```bash
cd engine && uv run pytest argot/tests/test_acceptance_runner.py -v 2>&1 | head -20
```

Expected: `ImportError` — `argot.acceptance.runner` does not exist yet.

- [ ] **Step 4: Write `engine/argot/acceptance/runner.py`**

```python
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from argot.tokenize import language_for_path, tokenize_lines
from argot.train import ModelBundle, train_model
from argot.validate import score_records, split_by_time

CATALOG_DIR = Path(__file__).parent / "catalog"
GATE_DELTA = 0.20
EPOCHS = 20


@dataclass
class ScopeConfig:
    name: str
    path_prefix: str
    paradigm: str


@dataclass
class FixtureSpec:
    name: str
    scope: str
    file: str
    hunk_start_line: int
    hunk_end_line: int
    is_break: bool
    rationale: str


@dataclass
class ScopeResult:
    name: str
    break_mean: float
    ctrl_mean: float
    delta: float
    passed: bool


@dataclass
class EntryResult:
    entry: str
    scope_results: list[ScopeResult]
    fixture_scores: list[dict[str, Any]]
    passed: bool


def load_scopes(entry_dir: Path) -> list[ScopeConfig]:
    data = json.loads((entry_dir / "scopes.json").read_text())
    return [ScopeConfig(**s) for s in data["scopes"]]


def load_manifest(entry_dir: Path) -> list[FixtureSpec]:
    data = json.loads((entry_dir / "manifest.json").read_text())
    return [FixtureSpec(**f) for f in data["fixtures"]]


def load_corpus(entry_dir: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with (entry_dir / "corpus.jsonl").open() as f:
        for line in f:
            if line.strip():
                records.append(json.loads(line))
    return records


def fixture_to_record(entry_dir: Path, spec: FixtureSpec) -> dict[str, Any]:
    fixture_path = entry_dir / spec.file
    source = fixture_path.read_text(encoding="utf-8")
    lines = source.splitlines()
    lang = language_for_path(str(fixture_path)) or "python"
    hunk_start = spec.hunk_start_line - 1
    hunk_end = spec.hunk_end_line - 1
    ctx_start = max(0, hunk_start - 20)
    ctx_tokens = tokenize_lines(lines, lang, ctx_start, hunk_start)
    hunk_tokens = tokenize_lines(lines, lang, hunk_start, hunk_end)
    return {
        "_repo": "acceptance-fixture",
        "author_date_iso": "0",
        "language": lang,
        "context_before": [{"text": t.text} for t in ctx_tokens],
        "context_after": [],
        "hunk_tokens": [{"text": t.text} for t in hunk_tokens],
    }


def run_entry(entry_dir: Path, epochs: int = EPOCHS) -> EntryResult:
    entry_name = entry_dir.name
    scopes = load_scopes(entry_dir)
    corpus = load_corpus(entry_dir)
    fixtures = load_manifest(entry_dir)

    bundles: dict[str, ModelBundle] = {}
    for scope in scopes:
        scope_records = [
            r for r in corpus
            if r.get("file_path", "").startswith(scope.path_prefix)
        ]
        if len(scope_records) < 10:
            raise RuntimeError(
                f"Entry {entry_name!r}, scope {scope.name!r}: "
                f"only {len(scope_records)} records (need ≥ 10)"
            )
        train_records, _ = split_by_time(scope_records, ratio=0.8)
        print(
            f"  [{scope.name}] {len(train_records)} train records, "
            f"training {epochs} epochs...",
            flush=True,
        )
        bundles[scope.name] = train_model(
            train_records, encoder="pretrained", epochs=epochs
        )

    fixture_scores: list[dict[str, Any]] = []
    for spec in fixtures:
        record = fixture_to_record(entry_dir, spec)
        scores = score_records(bundles[spec.scope], [record])
        score = scores[0] if scores else 0.0
        tag = "BREAK" if spec.is_break else "CTRL "
        print(
            f"  [{tag}][{spec.scope}] {spec.name:<40s} score={score:.4f}",
            flush=True,
        )
        fixture_scores.append(
            {
                "name": spec.name,
                "scope": spec.scope,
                "score": score,
                "is_break": spec.is_break,
            }
        )

    scope_results: list[ScopeResult] = []
    for scope in scopes:
        scope_fs = [f for f in fixture_scores if f["scope"] == scope.name]
        breaks = [f["score"] for f in scope_fs if f["is_break"]]
        controls = [f["score"] for f in scope_fs if not f["is_break"]]
        bm = sum(breaks) / len(breaks) if breaks else 0.0
        cm = sum(controls) / len(controls) if controls else 0.0
        delta = bm - cm
        scope_results.append(
            ScopeResult(
                name=scope.name,
                break_mean=bm,
                ctrl_mean=cm,
                delta=delta,
                passed=delta >= GATE_DELTA,
            )
        )

    return EntryResult(
        entry=entry_name,
        scope_results=scope_results,
        fixture_scores=fixture_scores,
        passed=all(s.passed for s in scope_results),
    )


def _write_markdown(result: EntryResult, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w") as f:
        f.write(f"# Acceptance Test: {result.entry}\n\n")
        f.write("| fixture | scope | score | type |\n|---|---|---|---|\n")
        for fs in result.fixture_scores:
            t = "break" if fs["is_break"] else "control"
            f.write(f"| {fs['name']} | {fs['scope']} | {fs['score']:.4f} | {t} |\n")
        f.write("\n")
        for sr in result.scope_results:
            gate = "GO ✓" if sr.passed else "NO-GO ✗"
            f.write(
                f"**[{sr.name}]** control={sr.ctrl_mean:.4f}  "
                f"break={sr.break_mean:.4f}  delta={sr.delta:.4f}  {gate}\n"
            )
        overall = "GO ✓" if result.passed else "NO-GO ✗"
        f.write(f"\n**Overall:** {overall}\n")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run acceptance tests on catalog entries"
    )
    parser.add_argument("--entry", help="Run a single catalog entry by name")
    parser.add_argument(
        "--catalog",
        default=str(CATALOG_DIR),
        help="Path to catalog directory (default: acceptance/catalog/)",
    )
    parser.add_argument(
        "--out",
        default="docs/research/scoring/acceptance",
        help="Directory to write results markdown",
    )
    parser.add_argument("--epochs", type=int, default=EPOCHS)
    args = parser.parse_args()

    catalog = Path(args.catalog)
    out_dir = Path(args.out)

    entries = (
        [catalog / args.entry]
        if args.entry
        else sorted(e for e in catalog.iterdir() if e.is_dir())
    )

    if not entries:
        print("No catalog entries found.", file=sys.stderr)
        sys.exit(1)

    all_passed = True
    for entry_dir in entries:
        print(f"\n=== {entry_dir.name} ===", flush=True)
        try:
            result = run_entry(entry_dir, epochs=args.epochs)
        except RuntimeError as e:
            print(f"  ERROR: {e}", file=sys.stderr)
            all_passed = False
            continue

        for sr in result.scope_results:
            gate = "GO ✓" if sr.passed else "NO-GO ✗"
            print(
                f"  [{sr.name}] control={sr.ctrl_mean:.4f}  "
                f"break={sr.break_mean:.4f}  delta={sr.delta:.4f}  {gate}"
            )
        _write_markdown(result, out_dir / f"{entry_dir.name}.md")
        if not result.passed:
            all_passed = False

    sys.exit(0 if all_passed else 1)


if __name__ == "__main__":
    main()
```

- [ ] **Step 5: Run tests — verify they pass**

```bash
cd engine && uv run pytest argot/tests/test_acceptance_runner.py -v 2>&1 | tail -20
```

Expected: all 7 tests pass.

- [ ] **Step 6: Run full test suite**

```bash
just test 2>&1 | tail -10
```

Expected: no regressions.

- [ ] **Step 7: Commit**

```bash
git add engine/argot/acceptance/ engine/argot/tests/test_acceptance_runner.py
git commit -m "feat(acceptance): add acceptance runner framework with catalog format"
```

---

## Task 3: ky corpus extraction

**Files:**
- Create: `engine/argot/acceptance/catalog/ky/corpus.jsonl`

- [ ] **Step 1: Extract ky records from bucket data and sample 600**

```python
# Run from repo root as: python engine/argot/acceptance/scripts/extract_ky_corpus.py
# Or just run this inline as a one-off script:
import json
import random
from pathlib import Path

src = Path(".argot/research/buckets-v2/small-ts.jsonl")
out = Path("engine/argot/acceptance/catalog/ky")
out.mkdir(parents=True, exist_ok=True)

records = []
with src.open() as f:
    for line in f:
        r = json.loads(line)
        if r.get("_repo") == "ky":
            records.append(r)

print(f"Found {len(records)} ky records")

rng = random.Random(42)
sample = rng.sample(records, min(600, len(records)))

with (out / "corpus.jsonl").open("w") as f:
    for r in sample:
        # Keep only the fields the runner needs
        f.write(json.dumps({
            "_repo": r["_repo"],
            "author_date_iso": r["author_date_iso"],
            "file_path": r.get("file_path", ""),
            "language": r["language"],
            "context_before": [{"text": t["text"]} for t in r["context_before"]],
            "context_after": [{"text": t["text"]} for t in r.get("context_after", [])],
            "hunk_tokens": [{"text": t["text"]} for t in r["hunk_tokens"]],
        }) + "\n")

print(f"Wrote {len(sample)} records to {out}/corpus.jsonl")
```

Run it:
```bash
cd /Users/damienmeur/projects/argot && uv run --package argot-engine python -c "
import json, random
from pathlib import Path

src = Path('.argot/research/buckets-v2/small-ts.jsonl')
out = Path('engine/argot/acceptance/catalog/ky')
out.mkdir(parents=True, exist_ok=True)

records = []
with src.open() as f:
    for line in f:
        r = json.loads(line)
        if r.get('_repo') == 'ky':
            records.append(r)

print(f'Found {len(records)} ky records')
rng = random.Random(42)
sample = rng.sample(records, min(600, len(records)))

with (out / 'corpus.jsonl').open('w') as f:
    for r in sample:
        f.write(json.dumps({
            '_repo': r['_repo'],
            'author_date_iso': r['author_date_iso'],
            'file_path': r.get('file_path', ''),
            'language': r['language'],
            'context_before': [{'text': t['text']} for t in r['context_before']],
            'context_after': [{'text': t['text']} for t in r.get('context_after', [])],
            'hunk_tokens': [{'text': t['text']} for t in r['hunk_tokens']],
        }) + '\n')

print(f'Wrote {len(sample)} records to {out}/corpus.jsonl')
"
```

Expected output: `Found 2431 ky records` then `Wrote 600 records to engine/argot/acceptance/catalog/ky/corpus.jsonl`

- [ ] **Step 2: Verify the file**

```bash
wc -l engine/argot/acceptance/catalog/ky/corpus.jsonl
python3 -c "
import json
with open('engine/argot/acceptance/catalog/ky/corpus.jsonl') as f:
    records = [json.loads(l) for l in f if l.strip()]
langs = set(r['language'] for r in records)
print(f'{len(records)} records, languages: {langs}')
print('sample file_path:', records[0].get('file_path'))
"
```

Expected: 600 records, language `typescript`, file_paths like `source/...`

- [ ] **Step 3: Commit**

```bash
git add engine/argot/acceptance/catalog/ky/corpus.jsonl
git commit -m "feat(acceptance): add ky corpus sample (600 records)"
```

---

## Task 4: ky deep analysis and scopes.json

**Files:**
- Create: `engine/argot/acceptance/catalog/ky/scopes.json`

This task requires reading ky's actual source code at the pinned SHA `e9eeb357` from `https://github.com/sindresorhus/ky`.

- [ ] **Step 1: Read ky's source to understand its paradigms**

Read these files from the ky repo (fetch from GitHub or use a local clone):
- `source/index.ts` — public API surface; note how the default export is created
- `source/errors.ts` — how errors are defined and typed
- `source/options.ts` — how options are normalized; note the types used
- `source/types.ts` — all public TypeScript types; note what's exported
- `source/utils.ts` — utility helpers; note the coding style
- `readme.md` — the intended usage API

While reading, document answers to:
1. Is this class-based or factory-based at the public API level?
2. How is error handling done? (try/catch, callbacks, `.catch()` chains, custom error classes?)
3. How are HTTP options structured? (flat object, builder pattern, chained methods?)
4. What TypeScript patterns are dominant? (generics, mapped types, conditional types, decorators?)
5. What is conspicuously absent? (no Node.js `require()`, no callbacks, no EventEmitter, no `this`-heavy patterns, no `async/await` in certain places?)
6. How are hooks/interceptors handled?

- [ ] **Step 2: Write `engine/argot/acceptance/catalog/ky/scopes.json`**

Single default scope. The `paradigm` field must be specific and evidence-based — not "functional TypeScript" but something like: "Fetch-first HTTP client using factory pattern (`ky.create()`), options-object configuration, typed generic responses (`.json<T>()`), `HTTPError` with `.response` for error handling — no callbacks, no Node.js APIs, no class instantiation at call sites, no `axios.create()` interceptor chains."

The exact wording must reflect what you actually found in the code, not assumptions. Format:

```json
{
  "scopes": [
    {
      "name": "default",
      "path_prefix": "",
      "paradigm": "<your evidence-based description from step 1>"
    }
  ]
}
```

- [ ] **Step 3: Commit**

```bash
git add engine/argot/acceptance/catalog/ky/scopes.json
git commit -m "feat(acceptance): add ky scope definition from codebase analysis"
```

---

## Task 5: ky fixtures and manifest

**Files:**
- Create: `engine/argot/acceptance/catalog/ky/fixtures/default/*.ts` (6–10 files)
- Create: `engine/argot/acceptance/catalog/ky/manifest.json`

Each fixture must trace back to a specific observation from Task 4's analysis. Target: 5–6 paradigm breaks, 2–3 controls. Rules:
- A break fixture contains code that ky would **never** write (e.g., callback-style, class-based, XHR, Node.js imports, explicit Promise constructor)
- A control fixture contains code that looks like ky's actual source (options normalization, fetch response handling, typed generics, error subclasses)
- Every fixture file should have ~20–40 lines of realistic context before the hunk, so the model has something to learn from

- [ ] **Step 1: Write break fixtures**

Based on paradigm observations from Task 4, create fixture files in `engine/argot/acceptance/catalog/ky/fixtures/default/`. Each file should represent one foreign paradigm.

Example paradigm breaks to consider (adapt based on what you actually found in step 4):
- `paradigm_break_xhr.ts` — using `XMLHttpRequest` instead of `fetch`-based API
- `paradigm_break_callback.ts` — Node.js-style `(err: Error | null, res?: Response) => void` callback
- `paradigm_break_class_client.ts` — `class HttpClient` with `constructor()` and instance methods (ky never exposes class instances to callers)
- `paradigm_break_explicit_promise.ts` — `new Promise((resolve, reject) => {...})` wrapping fetch (ky never wraps like this; it extends Response directly)
- `paradigm_break_interceptors.ts` — axios-style `client.interceptors.request.use(fn)` chaining (ky uses hooks array, not interceptor chains)

Example controls to consider:
- `control_options_normalization.ts` — code that looks like ky's own option normalization (spreading defaults, merging headers)
- `control_response_handling.ts` — code handling a fetch Response in ky's style (cloning, checking status, returning typed data)

For each fixture file: write realistic TypeScript code (not toy code). The hunk lines should be the foreign block. Context before the hunk should look like ky's normal code so the model has real context to work with.

Example structure for `paradigm_break_xhr.ts`:
```typescript
// In ky, all HTTP is via fetch(). XMLHttpRequest is never used.
// This file shows ky-style context before the hunk, then a foreign XHR block.

import type {Options} from './types.js';

// Context: normal ky-style option handling (model learns this is ky code)
export function normalizeOptions(input: Options): Required<Options> {
    return {
        method: input.method ?? 'GET',
        headers: new Headers(input.headers),
        retry: input.retry ?? 2,
        timeout: input.timeout ?? 10_000,
    };
}

export function buildSearchParams(searchParams: Options['searchParams']): string {
    if (!searchParams) return '';
    return new URLSearchParams(searchParams as Record<string, string>).toString();
}

// HUNK STARTS HERE — foreign XHR pattern
function legacyRequest(url: string, options: Options): Promise<Response> {
    return new Promise((resolve, reject) => {
        const xhr = new XMLHttpRequest();
        xhr.open(options.method ?? 'GET', url);
        xhr.onload = () => resolve(new Response(xhr.responseText));
        xhr.onerror = () => reject(new Error('Network error'));
        xhr.send();
    });
}
// HUNK ENDS HERE
```

- [ ] **Step 2: Write `engine/argot/acceptance/catalog/ky/manifest.json`**

For each fixture file, add an entry. `hunk_start_line` and `hunk_end_line` are 1-indexed. Count the actual lines in each fixture file to get the exact numbers.

```json
{
  "fixtures": [
    {
      "name": "paradigm_break_xhr",
      "scope": "default",
      "file": "fixtures/default/paradigm_break_xhr.ts",
      "hunk_start_line": <first line of XHR block>,
      "hunk_end_line": <last line of XHR block + 1>,
      "is_break": true,
      "rationale": "ky is entirely fetch-based; XMLHttpRequest is never used in the source"
    },
    ...
    {
      "name": "control_options_normalization",
      "scope": "default",
      "file": "fixtures/default/control_options_normalization.ts",
      "hunk_start_line": <first line of idiomatic block>,
      "hunk_end_line": <last line + 1>,
      "is_break": false,
      "rationale": "Options normalization is the core pattern in source/options.ts"
    }
  ]
}
```

- [ ] **Step 3: Commit**

```bash
git add engine/argot/acceptance/catalog/ky/
git commit -m "feat(acceptance): add ky fixtures and manifest from codebase analysis"
```

---

## Task 6: Run ky acceptance test and document results

**Files:**
- Create: `docs/research/scoring/acceptance/ky.md` (auto-generated by runner)

- [ ] **Step 1: Run the acceptance test on ky**

```bash
cd /Users/damienmeur/projects/argot && uv run --package argot-engine python -m argot.acceptance.runner \
    --entry ky \
    --catalog engine/argot/acceptance/catalog \
    --out docs/research/scoring/acceptance \
    --epochs 20 2>&1
```

Watch the output. For each fixture, note which score higher (breaks) vs lower (controls).

- [ ] **Step 2: Check the gate**

The runner exits 0 if all scopes pass (delta >= 0.20) and 1 if any fail. Check the printed per-scope summary.

If the gate passes (delta >= 0.20 for `default` scope): proceed to commit.

If the gate fails: investigate. Common causes:
- A break fixture scores below controls → check if the pattern actually exists in ky's corpus (the model has seen it). If yes, the fixture is wrong — replace it with a stronger break.
- The paradigm description in scopes.json is vague → this doesn't affect training, but it's a signal the fixture design is off.
- All scores are similar → the corpus may be too small or the JEPA head isn't finding any contrast. Re-examine the fixture files; make the break patterns more distinct from context.

Fix fixtures until the gate passes. Each fix is a targeted change to one or two fixture files, not a threshold change.

- [ ] **Step 3: Verify the output markdown**

```bash
cat docs/research/scoring/acceptance/ky.md
```

Expected structure:
```
# Acceptance Test: ky

| fixture | scope | score | type |
|---|---|---|---|
| paradigm_break_xhr | default | X.XXXX | break |
...

**[default]** control=X.XXXX  break=X.XXXX  delta=X.XXXX  GO ✓

**Overall:** GO ✓
```

- [ ] **Step 4: Commit results**

```bash
git add docs/research/scoring/acceptance/ky.md
git commit -m "feat(acceptance): ky acceptance test passes gate (delta >= 0.20)"
```
