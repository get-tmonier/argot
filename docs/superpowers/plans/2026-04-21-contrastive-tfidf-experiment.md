# Contrastive TF-IDF Experiment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a quick one-off experiment that applies the ast_contrastive log-ratio formula to raw code tokens (not AST treelets) on the FastAPI fixture catalog, to determine whether the contrast structure or the AST treelet vocabulary was the load-bearing innovation.

**Architecture:** Three new files — `build_token_reference.py` (CLI script to generate stdlib token frequencies), `experiments/contrastive_tfidf.py` (single-file experiment), `experiments/test_contrastive_tfidf.py` (two tests). No production wiring; no new scorer class in REGISTRY. The experiment reuses `tokenize_lines`, `fixture_to_record`, `load_manifest`, `auc_from_scores`, `_stdlib_root`, and `_walk_py_files` from existing infrastructure.

**Tech Stack:** Python 3.13, `argot.tokenize.tokenize_lines`, `argot.acceptance.runner`, `argot.research.signal.bootstrap.auc_from_scores`, `argot.research.signal.cli.build_reference` (for stdlib helpers).

---

## File Map

| Path | Action | Responsibility |
|---|---|---|
| `engine/argot/research/signal/cli/build_token_reference.py` | Create | Walk CPython stdlib, tokenize with `tokenize_lines`, emit `generic_tokens.json` |
| `engine/argot/research/reference/generic_tokens.json` | Generate (run script) | Stdlib token frequency table used as model_B |
| `engine/argot/research/signal/phase13/experiments/__init__.py` | Create (empty) | Make `experiments/` a proper Python package for mypy/pytest |
| `engine/argot/research/signal/phase13/experiments/contrastive_tfidf.py` | Create | Experiment: score FastAPI fixtures with token log-ratio, compute AUC, write report |
| `engine/argot/research/signal/phase13/experiments/test_contrastive_tfidf.py` | Create | Two tests: smoke (model_A=model_B→≈0) + end-to-end (no crash, 51 fixtures) |
| `docs/research/scoring/signal/phase13/experiments/contrastive_tfidf_2026-04-21.md` | Create | Comparison table, per-category AUC, interpretation paragraph |

---

## Task 1: build_token_reference.py + generate generic_tokens.json

**Files:**
- Create: `engine/argot/research/signal/cli/build_token_reference.py`
- Generate: `engine/argot/research/reference/generic_tokens.json`

### Background

`build_reference.py` walks the CPython stdlib and counts AST treelets. This script does the same thing but counts raw code tokens using `tokenize_lines`. The generated JSON is consumed as model_B by the experiment script. Reuse `_stdlib_root` and `_walk_py_files` from `build_reference` rather than reimplementing stdlib walking.

The `Lang` type alias is `Literal["typescript", "javascript", "python"]` defined in `argot.dataset`.

- [ ] **Step 1: Write the script**

Create `engine/argot/research/signal/cli/build_token_reference.py` with this exact content:

```python
"""Build a generic Python token frequency table from the CPython stdlib.

Usage:
    python -m argot.research.signal.cli.build_token_reference \\
        --out engine/argot/research/reference/generic_tokens.json
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from argot.dataset import Language as Lang
from argot.research.signal.cli.build_reference import _stdlib_root, _walk_py_files
from argot.tokenize import tokenize_lines

_TOP_N = 100_000
_MAX_BYTES = 10 * 1024 * 1024


def _build_counts(py_files: list[Path]) -> tuple[Counter[str], int]:
    counts: Counter[str] = Counter()
    total_files = 0
    lang: Lang = "python"
    for path in py_files:
        try:
            source = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        lines = source.splitlines(keepends=True)
        tokens = tokenize_lines(lines, lang, 0, len(lines))
        if tokens:
            counts.update(t.text for t in tokens)
            total_files += 1
    return counts, total_files


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Build generic token reference table")
    parser.add_argument("--out", required=True, help="Output JSON path")
    args = parser.parse_args(argv)

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    stdlib_root = _stdlib_root()
    print(f"Scanning {stdlib_root} ...", file=sys.stderr)
    py_files = _walk_py_files(stdlib_root)
    print(f"Found {len(py_files)} .py files", file=sys.stderr)

    counts, total_files = _build_counts(py_files)
    total_tokens = sum(counts.values())
    print(
        f"Extracted {total_tokens:,} tokens from {total_files} files",
        file=sys.stderr,
    )

    token_counts = dict(counts.most_common(_TOP_N))

    payload = {
        "version": 1,
        "token_counts": token_counts,
        "total_files": total_files,
        "total_tokens": total_tokens,
    }

    raw = json.dumps(payload, indent=2)
    if len(raw.encode()) > _MAX_BYTES:
        print(
            f"JSON exceeds 10 MB ({len(raw.encode()):,} bytes) — "
            f"already capped at top {_TOP_N:,}",
            file=sys.stderr,
        )

    out_path.write_text(raw, encoding="utf-8")
    print(f"Wrote {out_path} ({len(raw.encode()):,} bytes)", file=sys.stderr)


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run linting on the new script**

```bash
cd /Users/damienmeur/projects/argot
uv run --package argot-engine ruff check engine/argot/research/signal/cli/build_token_reference.py
uv run --package argot-engine mypy engine/argot/research/signal/cli/build_token_reference.py
```

Expected: no errors. If mypy complains about `_stdlib_root` or `_walk_py_files` being private, that's fine — they are intentionally reused across CLI scripts in the same package (validate_contrastive.py already does this).

- [ ] **Step 3: Generate generic_tokens.json**

```bash
uv run --package argot-engine python -m argot.research.signal.cli.build_token_reference \
    --out engine/argot/research/reference/generic_tokens.json
```

Expected stderr output (numbers will vary slightly):
```
Scanning /usr/lib/python3.13 ...
Found NNN .py files
Extracted X,XXX,XXX tokens from NNN files
Wrote engine/argot/research/reference/generic_tokens.json (Y,YYY,YYY bytes)
```

Then verify the JSON has the right shape:
```bash
python3 -c "
import json
d = json.load(open('engine/argot/research/reference/generic_tokens.json'))
print('version:', d['version'])
print('total_files:', d['total_files'])
print('total_tokens:', d['total_tokens'])
print('token_count keys:', len(d['token_counts']))
print('top 5:', list(d['token_counts'].items())[:5])
"
```

Expected: `version: 1`, `token_counts` has at most 100_000 keys, `total_tokens` is in the millions.

- [ ] **Step 4: Commit**

```bash
git add engine/argot/research/signal/cli/build_token_reference.py \
        engine/argot/research/reference/generic_tokens.json
git commit -m "research(phase-13-exp): build_token_reference script + generic_tokens.json"
```

---

## Task 2: Experiment script + tests

**Files:**
- Create: `engine/argot/research/signal/phase13/experiments/__init__.py`
- Create: `engine/argot/research/signal/phase13/experiments/contrastive_tfidf.py`
- Create: `engine/argot/research/signal/phase13/experiments/test_contrastive_tfidf.py`

### Background

The formula is `score(hunk) = max over hunk tokens t of [log(P_B(t) + ε) − log(P_A(t) + ε)]` where:
- `P_A(t) = model_a.get(t, 0) / total_a` — frequency in FastAPI control files (20 `.py` files at `acceptance/catalog/fastapi/fixtures/default/control_*.py`)
- `P_B(t) = model_b.get(t, 0) / total_b` — frequency in CPython stdlib (from `generic_tokens.json`)
- `ε = 1e-7`

If the hunk yields fewer than 3 tokens (via `record["hunk_tokens"]`), fall back to tokenizing the full fixture file (path is at `record["_fixture_path"]`).

The fixture records come from `fixture_to_record(fastapi_dir, spec)` which already returns tokenized hunks in `record["hunk_tokens"]` as `[{"text": str, "node_type": str, "start_line": int}, ...]`.

**Path math** for `experiments/contrastive_tfidf.py`:
- `Path(__file__).parent.parent.parent.parent.parent` = `engine/argot/` (package root)
- So `FASTAPI_DIR = Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"`
- And `REFERENCE_PATH = Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens.json"`

- [ ] **Step 1: Create the experiments package**

```bash
touch engine/argot/research/signal/phase13/experiments/__init__.py
```

- [ ] **Step 2: Write the failing tests first**

Create `engine/argot/research/signal/phase13/experiments/test_contrastive_tfidf.py`:

```python
"""Tests for phase13 contrastive_tfidf experiment."""

from __future__ import annotations

from pathlib import Path

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.research.signal.phase13.experiments.contrastive_tfidf import (
    _build_model_a,
    _load_model_b,
    score_records,
)

_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent
    / "acceptance"
    / "catalog"
    / "fastapi"
)


def test_smoke_model_a_equals_model_b() -> None:
    """When model_A = model_B, all log-ratio scores must be exactly 0."""
    specs = load_manifest(_FASTAPI_DIR)
    records = [fixture_to_record(_FASTAPI_DIR, spec) for spec in specs]
    model_b, total_b = _load_model_b()
    scores = score_records(records, model_b, total_b, model_b, total_b)
    assert max(abs(s) for s in scores) < 1e-9


def test_end_to_end_fastapi_no_crash() -> None:
    """End-to-end: loads exactly 31 breaks + 20 controls, scores all 51, returns floats."""
    specs = load_manifest(_FASTAPI_DIR)
    records = [fixture_to_record(_FASTAPI_DIR, spec) for spec in specs]
    assert sum(s.is_break for s in specs) == 31
    assert sum(not s.is_break for s in specs) == 20
    model_a, total_a = _build_model_a(_FASTAPI_DIR)
    model_b, total_b = _load_model_b()
    scores = score_records(records, model_a, total_a, model_b, total_b)
    assert len(scores) == 51
    assert all(isinstance(s, float) for s in scores)
```

- [ ] **Step 3: Run the tests — verify they fail with ImportError**

```bash
uv run --package argot-engine pytest \
    engine/argot/research/signal/phase13/experiments/test_contrastive_tfidf.py \
    -v 2>&1 | head -30
```

Expected: `ImportError: cannot import name '_build_model_a' from 'argot.research.signal.phase13.experiments.contrastive_tfidf'` (module doesn't exist yet).

- [ ] **Step 4: Write the experiment script**

Create `engine/argot/research/signal/phase13/experiments/contrastive_tfidf.py`:

```python
"""Phase 13 quick experiment: contrastive token TF-IDF on FastAPI.

Hypothesis: does the +0.28 AUC lift from ast_contrastive come from the contrastive
log-ratio alone, not AST treelets?  We apply the same formula to raw code tokens.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/contrastive_tfidf.py \\
        --out docs/research/scoring/signal/phase13/experiments/contrastive_tfidf_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.dataset import Language as Lang
from argot.research.signal.bootstrap import auc_from_scores
from argot.tokenize import tokenize_lines

_EPSILON = 1e-7
_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent
    / "acceptance"
    / "catalog"
    / "fastapi"
)
_REFERENCE_PATH = (
    Path(__file__).parent.parent.parent.parent
    / "reference"
    / "generic_tokens.json"
)


def _load_model_b() -> tuple[dict[str, int], int]:
    raw: dict[str, Any] = json.loads(_REFERENCE_PATH.read_text(encoding="utf-8"))
    token_counts: dict[str, int] = raw["token_counts"]
    total_tokens: int = raw["total_tokens"]
    return token_counts, total_tokens


def _build_model_a(fastapi_dir: Path) -> tuple[dict[str, int], int]:
    counts: Counter[str] = Counter()
    lang: Lang = "python"
    for path in sorted((fastapi_dir / "fixtures" / "default").glob("control_*.py")):
        source = path.read_text(encoding="utf-8", errors="replace")
        lines = source.splitlines(keepends=True)
        tokens = tokenize_lines(lines, lang, 0, len(lines))
        counts.update(t.text for t in tokens)
    total = sum(counts.values())
    return dict(counts), total


def _hunk_texts(record: dict[str, Any]) -> list[str]:
    texts = [tok["text"] for tok in record["hunk_tokens"]]
    if len(texts) < 3:
        fixture_path = Path(record["_fixture_path"])
        lang: Lang = "python"
        source = fixture_path.read_text(encoding="utf-8", errors="replace")
        lines = source.splitlines(keepends=True)
        texts = [t.text for t in tokenize_lines(lines, lang, 0, len(lines))]
    return texts


def _score_one(
    record: dict[str, Any],
    model_a: dict[str, int],
    total_a: int,
    model_b: dict[str, int],
    total_b: int,
) -> float:
    hunk = _hunk_texts(record)
    scores = [
        math.log(model_b.get(t, 0) / total_b + _EPSILON)
        - math.log(model_a.get(t, 0) / total_a + _EPSILON)
        for t in hunk
    ]
    return max(scores) if scores else 0.0


def score_records(
    records: list[dict[str, Any]],
    model_a: dict[str, int],
    total_a: int,
    model_b: dict[str, int],
    total_b: int,
) -> list[float]:
    return [_score_one(r, model_a, total_a, model_b, total_b) for r in records]


def _per_category_auc(
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
    ctrl_scores: list[float],
) -> dict[str, tuple[int, float]]:
    cat_breaks: dict[str, list[float]] = defaultdict(list)
    for s, b, cat in zip(scores, is_break, categories, strict=False):
        if b:
            cat_breaks[cat].append(s)
    return {
        cat: (len(cat_s), auc_from_scores(cat_s, ctrl_scores))
        for cat, cat_s in sorted(cat_breaks.items())
    }


def _interpretation(auc: float) -> str:
    if auc >= 0.85:
        return (
            f"AUC {auc:.4f} ≥ 0.85: the contrastive log-ratio formulation alone — applied "
            "to raw tokens rather than AST treelets — recovers most of the lift seen in "
            "ast_contrastive_max. The key innovation was the contrastive signal structure, "
            "not the AST treelet vocabulary. "
            "**Next step: pursue a contrastive MLM baseline** (e.g. CodeBERT "
            "log P_B(t) − log P_A(t)) to test whether a pre-trained token distribution "
            "outperforms the stdlib corpus."
        )
    if auc >= 0.76:
        return (
            f"AUC {auc:.4f} falls between 0.75 and 0.85. Both the contrastive formulation "
            "and the AST treelet vocabulary contribute to ast_contrastive's lift. The "
            "contrast is load-bearing but not sufficient alone; structural features also "
            "matter. Consider a contrastive MLM experiment alongside AST vocabulary "
            "refinement."
        )
    return (
        f"AUC {auc:.4f} ≤ 0.75: raw token contrast does not replicate ast_contrastive_max's "
        "lift. The AST treelet vocabulary was load-bearing — the contrast formula on surface "
        "tokens is insufficient. "
        "**Next step: structural features (AST treelets) must be preserved in any "
        "successor scorer; raw token MLM or TF-IDF contrastive approaches are unlikely "
        "to generalise.**"
    )


def _write_report(
    out: Path,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
    names: list[str],
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 — Contrastive TF-IDF Experiment (FastAPI, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | AUC |",
        "|---|---|",
        "| tfidf_anomaly (one-sided, existing) | 0.6968 |",
        "| ast_contrastive_max (AST + contrast) | 0.9742 |",
        f"| **contrastive_tfidf (tokens + contrast)** | **{overall_auc:.4f}** |",
        "",
        "## Per-Category AUC\n",
        "",
        "*(break category vs all controls)*\n",
        "",
        "| category | n_breaks | AUC |",
        "|---|---|---|",
    ]
    for cat, (n, cat_auc) in per_cat.items():
        lines.append(f"| {cat} | {n} | {cat_auc:.4f} |")
    lines += [
        "",
        "## Fixture Scores\n",
        "",
        "| fixture | category | is_break | score |",
        "|---|---|---|---|",
    ]
    for name, cat, b, s in zip(names, categories, is_break, scores, strict=False):
        lines.append(f"| {name} | {cat} | {b} | {s:.4f} |")
    lines += [
        "",
        "## Interpretation\n",
        "",
        _interpretation(overall_auc),
        "",
    ]
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written to {out}", flush=True)


def run(fastapi_dir: Path = _FASTAPI_DIR, out: Path | None = None) -> float:
    specs = load_manifest(fastapi_dir)
    records = [fixture_to_record(fastapi_dir, spec) for spec in specs]

    is_break = [spec.is_break for spec in specs]
    categories = [spec.category for spec in specs]
    names = [spec.name for spec in specs]

    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    assert n_breaks == 31, f"Expected 31 breaks, got {n_breaks}"
    assert n_ctrls == 20, f"Expected 20 controls, got {n_ctrls}"

    model_a, total_a = _build_model_a(fastapi_dir)
    model_b, total_b = _load_model_b()

    scores = score_records(records, model_a, total_a, model_b, total_b)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print("tfidf_anomaly (one-sided, existing):     0.6968")
    print("ast_contrastive_max (AST + contrast):    0.9742")
    print(f"contrastive_tfidf (tokens + contrast):   {overall_auc:.4f}")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Contrastive TF-IDF experiment")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    out = Path(args.out) if args.out else None
    run(out=out)


if __name__ == "__main__":
    main()
```

- [ ] **Step 5: Run the tests — verify they pass**

```bash
uv run --package argot-engine pytest \
    engine/argot/research/signal/phase13/experiments/test_contrastive_tfidf.py \
    -v
```

Expected:
```
PASSED test_smoke_model_a_equals_model_b
PASSED test_end_to_end_fastapi_no_crash
```

- [ ] **Step 6: Run linting and type checking**

```bash
uv run --package argot-engine ruff check \
    engine/argot/research/signal/phase13/experiments/
uv run --package argot-engine mypy \
    engine/argot/research/signal/phase13/experiments/
```

Expected: 0 errors. If mypy raises issues with `dict[str, Any]` from JSON, add `# type: ignore[assignment]` on the specific line (targeted, not broad).

- [ ] **Step 7: Commit**

```bash
git add engine/argot/research/signal/phase13/experiments/
git commit -m "research(phase-13-exp): contrastive_tfidf experiment script + tests"
```

---

## Task 3: Run the experiment and write the report

**Files:**
- Generate: `docs/research/scoring/signal/phase13/experiments/contrastive_tfidf_2026-04-21.md`

- [ ] **Step 1: Run the experiment with report output**

```bash
uv run --package argot-engine python \
    engine/argot/research/signal/phase13/experiments/contrastive_tfidf.py \
    --out docs/research/scoring/signal/phase13/experiments/contrastive_tfidf_2026-04-21.md
```

Expected stdout:
```
tfidf_anomaly (one-sided, existing):     0.6968
ast_contrastive_max (AST + contrast):    0.9742
contrastive_tfidf (tokens + contrast):   X.XXXX
Report written to docs/research/scoring/signal/phase13/experiments/contrastive_tfidf_2026-04-21.md
```

The AUC value is the key result. Note it for the next step.

- [ ] **Step 2: Verify the fixture counts are correct**

The stdout from the `run()` function should not raise the `AssertionError` guards (`Expected 31 breaks` / `Expected 20 controls`). If either fires, check that `load_manifest` is pointed at `_FASTAPI_DIR` correctly and that the manifest has not changed.

- [ ] **Step 3: Run just verify**

```bash
just verify
```

Expected: all checks pass (lint + format + typecheck + boundaries + knip + test). If a check fails, diagnose and fix before committing.

- [ ] **Step 4: Commit**

```bash
git add docs/research/scoring/signal/phase13/experiments/contrastive_tfidf_2026-04-21.md
git commit -m "research(phase-13-exp): contrastive_tfidf report (AUC=X.XXXX)"
```

Replace `X.XXXX` with the actual AUC value from Step 1.

---

## Self-Review

**Spec coverage:**
- ✅ `build_token_reference.py` — mirrors `build_reference.py` with `tokenize_lines`, top-100k cap, JSON `{version, token_counts, total_files, total_tokens}`
- ✅ `generic_tokens.json` at `engine/argot/research/reference/`
- ✅ `contrastive_tfidf.py` single-file experiment, NOT registered in REGISTRY
- ✅ model_A from 20 control `.py` files at `fastapi/fixtures/default/control_*.py`
- ✅ model_B from `generic_tokens.json`
- ✅ `max` aggregation, ε=1e-7
- ✅ Hunk fallback when < 3 tokens: tokenize full fixture file
- ✅ Assert 31 breaks + 20 controls before trusting AUC
- ✅ Comparison block: tfidf_anomaly / ast_contrastive_max / contrastive_tfidf
- ✅ Per-category AUC: each break category's scores vs all controls
- ✅ Smoke test: model_A=model_B → scores ≈ 0
- ✅ End-to-end test: no crash, 51 fixtures
- ✅ Report with interpretation paragraph keyed on AUC ≥ 0.85 vs ≤ 0.75
- ✅ No modification to `tfidf_anomaly.py`, `ast_contrastive.py`, `bakeoff.py`, REGISTRY
- ✅ Three commits: (1) reference script + JSON, (2) experiment + test, (3) report
- ✅ `just verify` passes

**Placeholder scan:** No TBDs, all code is complete.

**Type consistency:** `_load_model_b()` returns `tuple[dict[str, int], int]`; `_build_model_a()` returns `tuple[dict[str, int], int]`; `score_records()` takes `dict[str, int]` for both model args — consistent throughout.
