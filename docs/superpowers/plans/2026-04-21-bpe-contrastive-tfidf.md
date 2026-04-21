# BPE-Contrastive TF-IDF Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace word-level tokenization in the contrastive TF-IDF experiment with UnixCoder BPE subword tokenization to fix single-token max saturation and held-out vocabulary holes, then evaluate on FastAPI (gate ≥ 0.90) and click (gate ≥ 0.80 for success).

**Architecture:** Three new files mirror the existing `build_token_reference.py`, `contrastive_tfidf.py`, and `contrastive_tfidf_click.py`, with the only change being the tokenizer: `transformers.AutoTokenizer.from_pretrained("microsoft/unixcoder-base")` instead of `argot.tokenize.tokenize_lines`. The formula, manifest, fixture loading, and model_A/model_B structure are unchanged.

**Tech Stack:** Python 3.13, uv, transformers (AutoTokenizer), sklearn (roc_auc_score via existing `auc_from_scores`), pytest, mypy strict.

---

## File Map

| Action | Path |
|---|---|
| Create | `engine/argot/research/signal/cli/build_token_reference_bpe.py` |
| Create | `engine/argot/research/reference/generic_tokens_bpe.json` (generated artifact, not committed) |
| Create | `engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf.py` |
| Create | `engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_click.py` |
| Create | `engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py` |
| Create | `docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_fastapi_2026-04-21.md` |
| Create | `docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_click_2026-04-21.md` |

---

## Task 1: BPE reference builder + smoke test

**Files:**
- Create: `engine/argot/research/signal/cli/build_token_reference_bpe.py`
- Create: `engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py` (smoke test only at this stage)

- [ ] **Step 1: Write the failing smoke test**

```python
# engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py
"""Tests for phase13 BPE contrastive_tfidf experiment."""

from __future__ import annotations


def test_bpe_subword_count_exceeds_word_count() -> None:
    """BPE tokeniser splits identifiers into subwords: token count > whitespace-split count."""
    from transformers import AutoTokenizer  # type: ignore[import-untyped]

    tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")
    source = "paramtype_convert context_init_tail argparse_class_based"
    word_count = len(source.split())
    bpe_ids = tokenizer.encode(source, add_special_tokens=False)
    assert len(bpe_ids) > word_count, (
        f"Expected BPE count ({len(bpe_ids)}) > word count ({word_count})"
    )
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /Users/damienmeur/projects/argot
uv run --package argot-engine pytest engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py::test_bpe_subword_count_exceeds_word_count -v
```

Expected: FAIL with `ModuleNotFoundError` for `transformers` or `ImportError`. (If transformers is already installed, it may pass — that's fine, continue.)

- [ ] **Step 3: Write `build_token_reference_bpe.py`**

```python
# engine/argot/research/signal/cli/build_token_reference_bpe.py
"""Build a generic Python BPE-token frequency table from the CPython stdlib.

Mirrors build_token_reference.py but tokenises with UnixCoder BPE instead of
argot's word-level tokenizer.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/cli/build_token_reference_bpe.py \\
        --out engine/argot/research/reference/generic_tokens_bpe.json
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from transformers import AutoTokenizer  # type: ignore[import-untyped]

from argot.research.signal.cli.build_reference import _stdlib_root, _walk_py_files

_TOP_N = 100_000
_MAX_BYTES = 10 * 1024 * 1024
_MODEL_NAME = "microsoft/unixcoder-base"


def _build_counts(
    py_files: list[Path], tokenizer: object
) -> tuple[Counter[int], int]:
    counts: Counter[int] = Counter()
    total_files = 0
    for path in py_files:
        try:
            source = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        ids: list[int] = tokenizer.encode(source, add_special_tokens=False)  # type: ignore[union-attr]
        if ids:
            counts.update(ids)
            total_files += 1
    return counts, total_files


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Build generic BPE token reference table")
    parser.add_argument("--out", required=True, help="Output JSON path")
    args = parser.parse_args(argv)

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"Loading tokenizer {_MODEL_NAME} ...", file=sys.stderr)
    tokenizer = AutoTokenizer.from_pretrained(_MODEL_NAME)

    stdlib_root = _stdlib_root()
    print(f"Scanning {stdlib_root} ...", file=sys.stderr)
    py_files = _walk_py_files(stdlib_root)
    print(f"Found {len(py_files)} .py files", file=sys.stderr)

    counts, total_files = _build_counts(py_files, tokenizer)
    total_tokens = sum(counts.values())
    print(
        f"Extracted {total_tokens:,} BPE tokens from {total_files} files",
        file=sys.stderr,
    )

    # Store token IDs as strings so JSON keys are valid
    token_counts = {str(k): v for k, v in counts.most_common(_TOP_N)}

    payload = {
        "version": 1,
        "model": _MODEL_NAME,
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

- [ ] **Step 4: Generate the BPE reference file**

```bash
cd /Users/damienmeur/projects/argot
uv run --package argot-engine python engine/argot/research/signal/cli/build_token_reference_bpe.py \
    --out engine/argot/research/reference/generic_tokens_bpe.json
```

Expected: prints "Wrote engine/argot/research/reference/generic_tokens_bpe.json" to stderr. File appears at that path.

- [ ] **Step 5: Run the smoke test to verify it now passes**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py::test_bpe_subword_count_exceeds_word_count -v
```

Expected: PASS.

- [ ] **Step 6: Run `just verify`**

```bash
just verify
```

Expected: all checks pass. If mypy complains about `transformers` stubs, confirm the `# type: ignore[import-untyped]` on the import line is present — no global config changes.

- [ ] **Step 7: Commit**

```bash
git add engine/argot/research/signal/cli/build_token_reference_bpe.py \
        engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py
git commit -m "research(phase-13): BPE reference builder + smoke test"
```

---

## Task 2: FastAPI BPE experiment + report

**Files:**
- Create: `engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf.py`
- Extend: `engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py`
- Create: `docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_fastapi_2026-04-21.md`

- [ ] **Step 1: Add end-to-end test for FastAPI runner**

Append to `engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py`:

```python
from pathlib import Path

from argot.acceptance.runner import fixture_to_record, load_manifest

_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)


def test_bpe_end_to_end_fastapi_no_crash() -> None:
    """BPE FastAPI runner: loads 31 breaks + 20 controls, scores all, returns floats."""
    from argot.research.signal.phase13.experiments.bpe_contrastive_tfidf import (
        _build_model_a_bpe,
        _load_model_b_bpe,
        score_records_bpe,
    )

    specs = load_manifest(_FASTAPI_DIR)
    records = [fixture_to_record(_FASTAPI_DIR, spec) for spec in specs]
    assert sum(s.is_break for s in specs) == 31
    assert sum(not s.is_break for s in specs) == 20
    model_a, total_a = _build_model_a_bpe(_FASTAPI_DIR)
    model_b, total_b = _load_model_b_bpe()
    scores = score_records_bpe(records, model_a, total_a, model_b, total_b)
    assert len(scores) == 51
    assert all(isinstance(s, float) for s in scores)
```

- [ ] **Step 2: Run to verify it fails**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py::test_bpe_end_to_end_fastapi_no_crash -v
```

Expected: FAIL with `ImportError` (module not yet created).

- [ ] **Step 3: Write `bpe_contrastive_tfidf.py`**

```python
# engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf.py
"""Phase 13: BPE-contrastive TF-IDF on FastAPI.

Hypothesis: swapping word-level tokenization for UnixCoder BPE subword tokenization
fixes (1) single-token max saturation and (2) held-out vocabulary holes seen in
the word-token baseline (AUC 0.9847).

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf.py \\
        --out docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_fastapi_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from transformers import AutoTokenizer  # type: ignore[import-untyped]

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.research.signal.bootstrap import auc_from_scores

_EPSILON = 1e-7
_MODEL_NAME = "microsoft/unixcoder-base"
_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)
_REFERENCE_PATH = (
    Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens_bpe.json"
)


def _get_tokenizer() -> Any:
    return AutoTokenizer.from_pretrained(_MODEL_NAME)


def _load_model_b_bpe() -> tuple[dict[int, int], int]:
    raw: dict[str, Any] = json.loads(_REFERENCE_PATH.read_text(encoding="utf-8"))
    token_counts: dict[int, int] = {int(k): v for k, v in raw["token_counts"].items()}
    total_tokens: int = raw["total_tokens"]
    return token_counts, total_tokens


def _build_model_a_bpe(fastapi_dir: Path) -> tuple[dict[int, int], int]:
    tokenizer = _get_tokenizer()
    counts: Counter[int] = Counter()
    for path in sorted((fastapi_dir / "fixtures" / "default").glob("control_*.py")):
        source = path.read_text(encoding="utf-8", errors="replace")
        ids: list[int] = tokenizer.encode(source, add_special_tokens=False)
        counts.update(ids)
    total = sum(counts.values())
    return dict(counts), total


def _hunk_bpe_ids(record: dict[str, Any], tokenizer: Any) -> list[int]:
    fixture_path = Path(record["_fixture_path"])
    hunk_start = record.get("hunk_start_line", 0)
    hunk_end = record.get("hunk_end_line", 0)
    source = fixture_path.read_text(encoding="utf-8", errors="replace")
    lines = source.splitlines(keepends=True)
    if hunk_end > hunk_start:
        hunk_source = "".join(lines[hunk_start:hunk_end])
    else:
        hunk_source = source
    ids: list[int] = tokenizer.encode(hunk_source, add_special_tokens=False)
    if not ids:
        ids = tokenizer.encode(source, add_special_tokens=False)
    return ids


def _score_one_bpe(
    record: dict[str, Any],
    tokenizer: Any,
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> float:
    ids = _hunk_bpe_ids(record, tokenizer)
    scores = [
        math.log(model_b.get(i, 0) / total_b + _EPSILON)
        - math.log(model_a.get(i, 0) / total_a + _EPSILON)
        for i in ids
    ]
    return max(scores) if scores else 0.0


def score_records_bpe(
    records: list[dict[str, Any]],
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> list[float]:
    tokenizer = _get_tokenizer()
    return [_score_one_bpe(r, tokenizer, model_a, total_a, model_b, total_b) for r in records]


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


def _saturation_check(break_scores: list[float]) -> str:
    unique = len(set(break_scores))
    total = len(break_scores)
    if unique == 1:
        return f"**Max-token saturation present**: all {total} breaks share identical score {break_scores[0]:.4f}."
    return f"Max-token saturation resolved: {unique}/{total} unique break scores (word baseline had 1/8)."


def _interpretation(auc: float, saturation_note: str) -> str:
    band: str
    if auc >= 0.90:
        band = (
            f"AUC {auc:.4f} ≥ 0.90: FastAPI gate passed. "
            "BPE tokenization preserves the word-token baseline signal. Proceed to click."
        )
    elif auc >= 0.80:
        band = (
            f"AUC {auc:.4f} falls between 0.80 and 0.90: minor regression from word baseline (0.9847). "
            "BPE did not improve FastAPI; may still be worth running click to check cross-repo lift."
        )
    else:
        band = (
            f"AUC {auc:.4f} < 0.80: FastAPI gate FAILED — BPE tokenizer broke something. "
            "Stop and diagnose before proceeding to click."
        )
    return f"{band}\n\n{saturation_note}"


def _write_report(
    out: Path,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
    names: list[str],
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
) -> None:
    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    saturation_note = _saturation_check(break_scores)
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 — BPE Contrastive TF-IDF Experiment (FastAPI, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | tokenizer | AUC |",
        "|---|---|---|",
        "| contrastive_tfidf (word baseline) | argot tokenize_lines | 0.9847 |",
        f"| **bpe_contrastive_tfidf** | **UnixCoder BPE** | **{overall_auc:.4f}** |",
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
        _interpretation(overall_auc, saturation_note),
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

    model_a, total_a = _build_model_a_bpe(fastapi_dir)
    model_b, total_b = _load_model_b_bpe()

    scores = score_records_bpe(records, model_a, total_a, model_b, total_b)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print(f"contrastive_tfidf (word baseline):  0.9847")
    print(f"bpe_contrastive_tfidf (this run):   {overall_auc:.4f}")

    if overall_auc < 0.90:
        print("WARNING: FastAPI gate FAILED (< 0.90). Stop before running click.")
    else:
        print("FastAPI gate passed (≥ 0.90). Proceed to click.")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="BPE Contrastive TF-IDF experiment — FastAPI")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    out = Path(args.out) if args.out else None
    run(out=out)


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py::test_bpe_end_to_end_fastapi_no_crash -v
```

Expected: PASS.

- [ ] **Step 5: Run the FastAPI experiment and save the report**

```bash
uv run --package argot-engine python \
    engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf.py \
    --out docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_fastapi_2026-04-21.md
```

Expected stdout: prints the two comparison lines and either "FastAPI gate passed" or "WARNING: FastAPI gate FAILED".

**Gate check**: If AUC < 0.90, stop here and report the regression. Do **not** proceed to Task 3 (click runner). Write a brief note at the bottom of the FastAPI report explaining what likely broke (e.g., `_hunk_bpe_ids` fell back to full-file encoding, or BPE IDs in model_B don't overlap with model_A).

- [ ] **Step 6: Run `just verify`**

```bash
just verify
```

Expected: all checks pass.

- [ ] **Step 7: Commit**

```bash
git add engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf.py \
        engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py \
        docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_fastapi_2026-04-21.md
git commit -m "research(phase-13): BPE contrastive-tfidf FastAPI runner + report"
```

---

## Task 3: Click BPE experiment + report

**Files:**
- Create: `engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_click.py`
- Extend: `engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py`
- Create: `docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_click_2026-04-21.md`

*Only proceed here if FastAPI AUC ≥ 0.90.*

- [ ] **Step 1: Add end-to-end test for click runner**

Append to `engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py`:

```python
from argot.acceptance.runner import FixtureSpec

_FIXTURE_DIR = (
    Path(__file__).parent.parent / "tier3_fixtures" / "click"
)
_MANIFEST_PATH = _FIXTURE_DIR / "manifest_matched.json"


def test_bpe_end_to_end_click_no_crash() -> None:
    """BPE click runner: loads 8 breaks + 10 controls, scores all, returns floats."""
    import json

    from argot.acceptance.runner import fixture_to_record
    from argot.research.signal.phase13.experiments.bpe_contrastive_tfidf_click import (
        _build_model_a_bpe_click,
        _load_model_b_bpe,
        score_records_bpe,
    )

    manifest = json.loads(_MANIFEST_PATH.read_text())
    records = []
    is_break_list = []
    for f in manifest["fixtures"]:
        spec = FixtureSpec(
            name=f["name"],
            scope="default",
            file=f["file"],
            hunk_start_line=f["hunk_start_line"],
            hunk_end_line=f["hunk_end_line"],
            is_break=f["is_break"],
            rationale=f.get("rationale", ""),
            category=f.get("category", "control" if not f["is_break"] else "break"),
        )
        records.append(fixture_to_record(_FIXTURE_DIR, spec, "file_only"))
        is_break_list.append(spec.is_break)
    assert sum(is_break_list) == 8
    assert sum(not b for b in is_break_list) == 10
    # model_A requires click_dir — skip if not available
    import os
    click_dir = Path(os.environ.get("CLICK_DIR", "/tmp/click-clone"))
    if not click_dir.is_dir():
        import pytest
        pytest.skip("CLICK_DIR not available; set env var to run this test")
    model_a, total_a = _build_model_a_bpe_click(click_dir)
    model_b, total_b = _load_model_b_bpe()
    scores = score_records_bpe(records, model_a, total_a, model_b, total_b)
    assert len(scores) == 18
    assert all(isinstance(s, float) for s in scores)
```

- [ ] **Step 2: Run to verify it fails (or skips)**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py::test_bpe_end_to_end_click_no_crash -v
```

Expected: SKIP (click-clone not at `/tmp/click-clone`) or FAIL with ImportError.

- [ ] **Step 3: Write `bpe_contrastive_tfidf_click.py`**

```python
# engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_click.py
"""Phase 13: BPE-contrastive TF-IDF on click (tier3 matched).

Mirrors bpe_contrastive_tfidf.py but uses click's 18-fixture matched manifest.
Hypothesis: BPE subword tokenization closes vocabulary holes in cross-repo evaluation.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_click.py \\
        --click-dir /tmp/click-clone \\
        --out docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_click_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from transformers import AutoTokenizer  # type: ignore[import-untyped]

from argot.acceptance.runner import FixtureSpec, fixture_to_record
from argot.research.signal.bootstrap import auc_from_scores

_EPSILON = 1e-7
_MODEL_NAME = "microsoft/unixcoder-base"
_FIXTURE_DIR = Path(__file__).parent.parent / "tier3_fixtures" / "click"
_MANIFEST_PATH = _FIXTURE_DIR / "manifest_matched.json"
_REFERENCE_PATH = (
    Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens_bpe.json"
)
_HELD_OUT: frozenset[str] = frozenset({"decorators.py", "types.py", "core.py"})


def _get_tokenizer() -> Any:
    return AutoTokenizer.from_pretrained(_MODEL_NAME)


def _load_model_b_bpe() -> tuple[dict[int, int], int]:
    raw: dict[str, Any] = json.loads(_REFERENCE_PATH.read_text(encoding="utf-8"))
    token_counts: dict[int, int] = {int(k): v for k, v in raw["token_counts"].items()}
    total_tokens: int = raw["total_tokens"]
    return token_counts, total_tokens


def _build_model_a_bpe_click(click_dir: Path) -> tuple[dict[int, int], int]:
    tokenizer = _get_tokenizer()
    counts: Counter[int] = Counter()
    src = click_dir / "src" / "click"
    for path in sorted(src.glob("*.py")):
        if path.name in _HELD_OUT:
            continue
        source = path.read_text(encoding="utf-8", errors="replace")
        ids: list[int] = tokenizer.encode(source, add_special_tokens=False)
        counts.update(ids)
    total = sum(counts.values())
    return dict(counts), total


def _load_fixtures() -> tuple[list[dict[str, Any]], list[bool], list[str], list[str]]:
    manifest = json.loads(_MANIFEST_PATH.read_text())
    records: list[dict[str, Any]] = []
    is_break: list[bool] = []
    names: list[str] = []
    categories: list[str] = []
    for f in manifest["fixtures"]:
        spec = FixtureSpec(
            name=f["name"],
            scope="default",
            file=f["file"],
            hunk_start_line=f["hunk_start_line"],
            hunk_end_line=f["hunk_end_line"],
            is_break=f["is_break"],
            rationale=f.get("rationale", ""),
            category=f.get("category", "control" if not f["is_break"] else "break"),
        )
        records.append(fixture_to_record(_FIXTURE_DIR, spec, "file_only"))
        is_break.append(spec.is_break)
        names.append(spec.name)
        categories.append(spec.category)
    return records, is_break, names, categories


def _hunk_bpe_ids(record: dict[str, Any], tokenizer: Any) -> list[int]:
    fixture_path = Path(record["_fixture_path"])
    hunk_start = record.get("hunk_start_line", 0)
    hunk_end = record.get("hunk_end_line", 0)
    source = fixture_path.read_text(encoding="utf-8", errors="replace")
    lines = source.splitlines(keepends=True)
    if hunk_end > hunk_start:
        hunk_source = "".join(lines[hunk_start:hunk_end])
    else:
        hunk_source = source
    ids: list[int] = tokenizer.encode(hunk_source, add_special_tokens=False)
    if not ids:
        ids = tokenizer.encode(source, add_special_tokens=False)
    return ids


def _score_one_bpe(
    record: dict[str, Any],
    tokenizer: Any,
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> float:
    ids = _hunk_bpe_ids(record, tokenizer)
    scores = [
        math.log(model_b.get(i, 0) / total_b + _EPSILON)
        - math.log(model_a.get(i, 0) / total_a + _EPSILON)
        for i in ids
    ]
    return max(scores) if scores else 0.0


def score_records_bpe(
    records: list[dict[str, Any]],
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> list[float]:
    tokenizer = _get_tokenizer()
    return [_score_one_bpe(r, tokenizer, model_a, total_a, model_b, total_b) for r in records]


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


def _saturation_check(break_scores: list[float]) -> str:
    unique = len(set(break_scores))
    total = len(break_scores)
    if unique == 1:
        return (
            f"**Max-token saturation still present**: all {total} breaks share identical "
            f"score {break_scores[0]:.4f}. BPE did not resolve saturation."
        )
    return (
        f"Max-token saturation resolved: {unique}/{total} unique break scores "
        f"(word baseline had 1/8 unique scores — all 8 identical at 8.4418)."
    )


def _interpretation(auc: float, saturation_note: str) -> str:
    if auc >= 0.80:
        band = (
            f"AUC {auc:.4f} ≥ 0.80: **SUCCESS**. BPE subword tokenization fixed the vocabulary "
            "holes and saturation issues from the word-token baseline (0.7000). "
            "**Recommend promoting BPE-contrastive-tfidf as Phase 13 winner.**"
        )
    elif auc >= 0.70:
        band = (
            f"AUC {auc:.4f} in [0.70, 0.80): partial lift over word baseline (0.7000) or at parity. "
            "BPE tokens are not the primary bottleneck; contrastive-MLM is justified as next step."
        )
    else:
        band = (
            f"AUC {auc:.4f} < 0.70: regression from word baseline (0.7000). "
            "BPE tokens made things worse — vocabulary holes are not the bottleneck. "
            "Recommend a context-aware approach (conditional distributions over context windows)."
        )
    return f"{band}\n\n{saturation_note}"


def _write_report(
    out: Path,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
    names: list[str],
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
) -> None:
    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    saturation_note = _saturation_check(break_scores)
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 — BPE Contrastive TF-IDF on Click (Tier 3 Matched, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | corpus | tokenizer | AUC |",
        "|---|---|---|---|",
        "| contrastive_tfidf (word baseline) | click (v2 matched) | argot tokenize_lines | 0.7000 |",
        f"| **bpe_contrastive_tfidf** | **click (v2 matched)** | **UnixCoder BPE** | **{overall_auc:.4f}** |",
        "",
        "## Per-Category AUC\n",
        "",
        "*(break category vs all 10 controls)*\n",
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
        _interpretation(overall_auc, saturation_note),
        "",
    ]
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written to {out}", flush=True)


def run(*, click_dir: Path, out: Path | None = None) -> float:
    records, is_break, names, categories = _load_fixtures()

    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    assert n_breaks == 8, f"Expected 8 breaks, got {n_breaks}"
    assert n_ctrls == 10, f"Expected 10 controls, got {n_ctrls}"

    model_a, total_a = _build_model_a_bpe_click(click_dir)
    model_b, total_b = _load_model_b_bpe()

    scores = [
        _score_one_bpe(r, _get_tokenizer(), model_a, total_a, model_b, total_b) for r in records
    ]

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print("contrastive_tfidf on click (word baseline):    0.7000")
    print(f"bpe_contrastive_tfidf on click (this run):     {overall_auc:.4f}")

    saturation_unique = len(set(break_scores))
    print(f"Break score unique values: {saturation_unique}/8 (word baseline: 1/8, all identical)")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="BPE Contrastive TF-IDF experiment — click")
    parser.add_argument("--click-dir", default="/tmp/click-clone", help="Path to click repo")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    run(click_dir=Path(args.click_dir), out=Path(args.out) if args.out else None)


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run the click experiment and save the report**

```bash
uv run --package argot-engine python \
    engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_click.py \
    --click-dir /tmp/click-clone \
    --out docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_click_2026-04-21.md
```

Expected: prints comparison rows and "Break score unique values: X/8".

**Gate interpretation:**
- AUC ≥ 0.80 → Success; report recommends promoting BPE-contrastive-tfidf as Phase 13 winner.
- AUC 0.70–0.80 → Partial lift; report states contrastive-MLM is next.
- AUC < 0.70 → Regression; report recommends context-aware approach.

- [ ] **Step 5: Run all BPE tests**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py -v
```

Expected: `test_bpe_subword_count_exceeds_word_count` and `test_bpe_end_to_end_fastapi_no_crash` PASS; `test_bpe_end_to_end_click_no_crash` SKIP (unless CLICK_DIR set).

- [ ] **Step 6: Run `just verify`**

```bash
just verify
```

Expected: all checks pass.

- [ ] **Step 7: Commit**

```bash
git add engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_click.py \
        engine/argot/research/signal/phase13/experiments/test_bpe_contrastive_tfidf.py \
        docs/research/scoring/signal/phase13/experiments/bpe_contrastive_tfidf_click_2026-04-21.md
git commit -m "research(phase-13): BPE contrastive-tfidf click runner + report"
```
