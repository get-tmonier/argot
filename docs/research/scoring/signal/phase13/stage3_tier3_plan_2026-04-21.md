# Phase 13 Stage 3 — Tier 3 Cross-Domain Validation (click)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Confirm `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation="max")` detects paradigm breaks on a **non-FastAPI Python repo** (the `click` CLI library) using **synthetic breaks** — without any FastAPI-specific tuning — before committing to a real second catalog or the TypeScript AST port.

**Architecture:** Build model_A from click's own source (minus held-out controls). Construct synthetic breaks as valid Python modules that import click and look click-ish, but whose hunk body is written in an *opposing CLI paradigm* (argparse class-based, OptionParser, raw `sys.argv`, docopt). If AUC stays comfortably above chance, the contrastive mechanism generalises; if it collapses, the FastAPI result was fixture-tuned. Mirrors the existing `validate_contrastive.py` (Tier 1+2) infrastructure pattern.

**Tech Stack:** Python 3.13, `uv`, `pytest`, existing `ContrastiveAstTreeletScorer`, existing `fixture_to_record` / `auc_from_scores` / `_walk_py_files` utilities. No new dependencies.

**Victory gate (pre-registered — do not move post-hoc):**
| AUC | Interpretation |
|---|---|
| ≥ 0.80 | PASS — method generalises; proceed to Stage 4 (TS AST port or real second catalog) |
| 0.65 – 0.80 | MIXED — investigate per-category before any promotion decision |
| < 0.65 | FAIL — method is FastAPI-tuned; do not promote |

Synthetic breaks are hand-crafted, so the bar is slightly lower than FastAPI's 0.97. The key test is: do breaks score materially higher than controls, without any click-specific tuning?

---

## File Structure

**New files:**
- `engine/argot/research/signal/phase13/tier3_fixtures/click/manifest.json` — manifest listing all 18 fixtures
- `engine/argot/research/signal/phase13/tier3_fixtures/click/controls/*.py` — 10 click source files held out from model_A
- `engine/argot/research/signal/phase13/tier3_fixtures/click/breaks/*.py` — 8 synthetic break files
- `engine/argot/research/signal/phase13/validate_tier3_click.py` — validation runner (mirrors `validate_contrastive.py`)
- `engine/argot/research/signal/phase13/test_validate_tier3_click.py` — pytest tests
- `docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md` — final report (written by the runner)

**Modified files:** none. This is additive research code; no production wiring changes.

**External prerequisite (not checked in):** a local click clone at `/tmp/click-clone`, tag `8.1.7`:
```bash
git clone https://github.com/pallets/click /tmp/click-clone && git -C /tmp/click-clone checkout 8.1.7
```
The runner takes a `--click-dir` arg so the path is not hardcoded.

---

### Task 1: Scaffold fixture directory + manifest schema

**Files:**
- Create: `engine/argot/research/signal/phase13/tier3_fixtures/click/manifest.json`
- Create: `engine/argot/research/signal/phase13/tier3_fixtures/click/controls/.gitkeep`
- Create: `engine/argot/research/signal/phase13/tier3_fixtures/click/breaks/.gitkeep`

- [ ] **Step 1: Create an empty manifest with the expected schema**

Write exactly this JSON (we will append entries in later tasks):

```json
{
  "source_repo": "https://github.com/pallets/click",
  "source_tag": "8.1.7",
  "fixtures": []
}
```

- [ ] **Step 2: Create the two subdirectories with .gitkeep files**

Both subdirs must exist so Task 4's runner can glob them without error.

- [ ] **Step 3: Commit**

```bash
git add engine/argot/research/signal/phase13/tier3_fixtures/click/
git commit -m "research(phase-13-s3): scaffold tier3 click fixture dirs"
```

---

### Task 2: Copy 10 held-out controls from click

**Files:**
- Create: 10 files in `engine/argot/research/signal/phase13/tier3_fixtures/click/controls/`
- Modify: `engine/argot/research/signal/phase13/tier3_fixtures/click/manifest.json`

Pick control files that span click's major surface area (decorators, types, parsing, context, termui, utils) so model_A evaluation is not dominated by one style.

**Choose these 10 files from `/tmp/click-clone/src/click/`:**
1. `decorators.py`
2. `types.py`
3. `core.py`
4. `termui.py`
5. `utils.py`
6. `parser.py`
7. `formatting.py`
8. `exceptions.py`
9. `globals.py`
10. `shell_completion.py`

- [ ] **Step 1: Write the failing test**

Create `engine/argot/research/signal/phase13/test_validate_tier3_click.py` (first version):

```python
"""Tests for Tier 3 click validation."""
from __future__ import annotations
import json
from pathlib import Path

FIXTURE_DIR = Path(__file__).parent / "tier3_fixtures" / "click"
CONTROLS_DIR = FIXTURE_DIR / "controls"
BREAKS_DIR = FIXTURE_DIR / "breaks"
MANIFEST_PATH = FIXTURE_DIR / "manifest.json"


def test_manifest_lists_all_fixtures() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    control_entries = [f for f in manifest["fixtures"] if not f["is_break"]]
    break_entries = [f for f in manifest["fixtures"] if f["is_break"]]
    assert len(control_entries) == 10, f"expected 10 controls, got {len(control_entries)}"
    assert len(break_entries) == 8, f"expected 8 breaks, got {len(break_entries)}"


def test_all_fixture_files_exist() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        path = FIXTURE_DIR / f["file"]
        assert path.exists(), f"fixture file missing: {path}"


def test_control_files_parse_as_python() -> None:
    import ast
    for path in CONTROLS_DIR.glob("*.py"):
        ast.parse(path.read_text())
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd /Users/damienmeur/projects/argot
uv run --package argot-engine pytest engine/argot/research/signal/phase13/test_validate_tier3_click.py -v
```
Expected: FAIL — manifest has 0 fixtures, not 18.

- [ ] **Step 3: Copy the 10 control files and update the manifest**

Copy each file verbatim (preserve line endings, no modifications):
```bash
for f in decorators.py types.py core.py termui.py utils.py parser.py formatting.py exceptions.py globals.py shell_completion.py; do
  cp "/tmp/click-clone/src/click/$f" "engine/argot/research/signal/phase13/tier3_fixtures/click/controls/$f"
done
```

Then rewrite `manifest.json` to include one entry per control. For controls, use `hunk_start_line = 1` and `hunk_end_line = <line count of the file>` (i.e., the whole file is the hunk — we want the scorer to evaluate the full body; the scorer's fallback will handle it anyway).

Example entry:
```json
{
  "name": "control_click_decorators",
  "file": "controls/decorators.py",
  "hunk_start_line": 1,
  "hunk_end_line": 450,
  "is_break": false,
  "rationale": "Canonical click decorator definitions — should score as in-distribution"
}
```

Get line counts with `wc -l` on each file and use those values.

- [ ] **Step 4: Run the test to verify it passes (partially — only control count matches)**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/test_validate_tier3_click.py::test_control_files_parse_as_python -v
```
Expected: PASS.

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/test_validate_tier3_click.py::test_manifest_lists_all_fixtures -v
```
Expected: FAIL — 10 controls but 0 breaks. That's expected; Task 3 adds breaks.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/research/signal/phase13/tier3_fixtures/click/controls/ \
        engine/argot/research/signal/phase13/tier3_fixtures/click/manifest.json \
        engine/argot/research/signal/phase13/test_validate_tier3_click.py
git commit -m "research(phase-13-s3): copy 10 click control fixtures + manifest"
```

---

### Task 3: Write 8 synthetic break fixtures

**Files:**
- Create: 8 files in `engine/argot/research/signal/phase13/tier3_fixtures/click/breaks/`
- Modify: `engine/argot/research/signal/phase13/tier3_fixtures/click/manifest.json`

**Paradigm coverage (8 breaks across 4 categories):**
- `break_argparse_class_based_1.py`, `break_argparse_class_based_2.py`, `break_argparse_class_based_3.py` — `ArgumentParser` subclasses, `add_argument` calls, `args.xxx` access
- `break_optparse_deprecated_1.py`, `break_optparse_deprecated_2.py` — `optparse.OptionParser`, `make_option`, `parser.add_option`
- `break_raw_sys_argv_1.py`, `break_raw_sys_argv_2.py` — manual `sys.argv` iteration, `if arg == "--foo":` chains
- `break_docopt_style_1.py` — `docopt.docopt(__doc__)` with a big docstring

**Each break file structure** (valid Python, ~80 lines):
```python
"""<brief description — break fixture, not for import>."""
from __future__ import annotations
import click  # decoy import to look click-like

# Decoy click command — NOT inside the hunk range
@click.command()
@click.option("--name", default="world")
def decoy(name: str) -> None:
    click.echo(f"Hello, {name}")

# ---- HUNK STARTS HERE (record the actual start line) ----
# <opposing-paradigm code, ~40 lines — argparse/optparse/raw argv/docopt>
# ---- HUNK ENDS HERE ----

if __name__ == "__main__":
    decoy()
```

The manifest entry's `hunk_start_line` / `hunk_end_line` must point to the opposing-paradigm block (NOT the click decoy). This mimics the FastAPI fixture pattern where the hunk is the foreign region.

- [ ] **Step 1: Write the failing test additions**

Append to `engine/argot/research/signal/phase13/test_validate_tier3_click.py`:

```python
def test_break_files_have_click_import() -> None:
    """Breaks must look click-ish at the file level, not just the hunk."""
    for path in BREAKS_DIR.glob("*.py"):
        assert "import click" in path.read_text(), f"{path.name} missing click import"


def test_break_hunk_ranges_are_valid() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if not f["is_break"]:
            continue
        path = FIXTURE_DIR / f["file"]
        line_count = len(path.read_text().splitlines())
        assert 1 <= f["hunk_start_line"] <= f["hunk_end_line"] <= line_count, \
            f"invalid hunk range in {f['name']}: {f['hunk_start_line']}-{f['hunk_end_line']} for {line_count}-line file"


def test_break_files_parse_as_python() -> None:
    import ast
    for path in BREAKS_DIR.glob("*.py"):
        ast.parse(path.read_text())
```

- [ ] **Step 2: Run tests to verify new ones fail**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/test_validate_tier3_click.py -v
```
Expected: 3 new tests FAIL (no break files yet), manifest count test still FAIL.

- [ ] **Step 3: Hand-write 8 break files + update manifest**

Write each file to match the structure above. For the hunk bodies, use these *minimum* constructs to ensure the opposing-paradigm signal is strong:

**argparse class-based** (all 3 variants should use all of):
```python
class MyCommandParser(argparse.ArgumentParser):
    def __init__(self):
        super().__init__(description="...")
        self.add_argument("--name", type=str, required=True)
        self.add_argument("-v", "--verbose", action="store_true")

parser = MyCommandParser()
args = parser.parse_args()
if args.verbose:
    print(args.name)
```

**optparse** (both variants):
```python
from optparse import OptionParser, make_option
parser = OptionParser(option_list=[
    make_option("-f", "--file", dest="filename"),
    make_option("-v", "--verbose", action="store_true"),
])
(options, args) = parser.parse_args()
```

**raw sys.argv** (both variants):
```python
import sys
args = sys.argv[1:]
i = 0
while i < len(args):
    if args[i] == "--name":
        name = args[i + 1]
        i += 2
    elif args[i] == "--verbose":
        verbose = True
        i += 1
    else:
        sys.exit(f"unknown arg: {args[i]}")
```

**docopt** (one variant):
```python
"""Usage: mycli [--verbose] <name>"""
from docopt import docopt
opts = docopt(__doc__)
if opts["--verbose"]:
    print(opts["<name>"])
```

For each break, add a manifest entry with precise `hunk_start_line` / `hunk_end_line` bracketing only the opposing-paradigm block. Use `category` = one of `{argparse_class, optparse, raw_argv, docopt}`.

- [ ] **Step 4: Run all tests to verify pass**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/test_validate_tier3_click.py -v
```
Expected: all 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/research/signal/phase13/tier3_fixtures/click/breaks/ \
        engine/argot/research/signal/phase13/tier3_fixtures/click/manifest.json \
        engine/argot/research/signal/phase13/test_validate_tier3_click.py
git commit -m "research(phase-13-s3): add 8 synthetic click break fixtures"
```

---

### Task 4: Write the validation runner

**Files:**
- Create: `engine/argot/research/signal/phase13/validate_tier3_click.py`
- Modify: `engine/argot/research/signal/phase13/test_validate_tier3_click.py`

The runner mirrors the existing Tier-2 runner (`validate_contrastive.py`) in structure, but uses a manifest-driven fixture loader instead of FastAPI's acceptance loader.

- [ ] **Step 1: Write the failing test**

Append to `test_validate_tier3_click.py`:

```python
def test_runner_produces_auc_report(tmp_path: Path) -> None:
    """End-to-end: load manifest, build model_A, score, compute AUC, write report."""
    import os
    click_dir = Path(os.environ.get("TIER3_CLICK_DIR", "/tmp/click-clone"))
    if not click_dir.exists():
        import pytest
        pytest.skip(f"click clone not found at {click_dir}; set TIER3_CLICK_DIR env var")

    from argot.research.signal.phase13.validate_tier3_click import run

    out = tmp_path / "report.md"
    result = run(click_dir=click_dir, out=out)

    assert "auc" in result
    assert 0.0 <= result["auc"] <= 1.0
    assert out.exists()
    assert "Verdict" in out.read_text()
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/test_validate_tier3_click.py::test_runner_produces_auc_report -v
```
Expected: FAIL — `validate_tier3_click` module does not exist.

- [ ] **Step 3: Write the runner**

Create `engine/argot/research/signal/phase13/validate_tier3_click.py`:

```python
"""Phase 13 Stage 3 Tier 3 — cross-domain validation on click.

Builds model_A from click source files (excluding the 10 fixture controls),
scores 18 fixtures (10 controls + 8 synthetic breaks), computes AUC, writes
a markdown report.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/validate_tier3_click.py \\
        --click-dir /tmp/click-clone \\
        --out docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from argot.acceptance.runner import FixtureSpec, fixture_to_record
from argot.research.signal.bootstrap import auc_from_scores
from argot.research.signal.scorers.ast_contrastive import ContrastiveAstTreeletScorer

FIXTURE_DIR = Path(__file__).parent / "tier3_fixtures" / "click"
MANIFEST_PATH = FIXTURE_DIR / "manifest.json"


def _load_fixtures() -> tuple[list[dict[str, Any]], list[bool], list[str], list[str]]:
    manifest = json.loads(MANIFEST_PATH.read_text())
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
        records.append(fixture_to_record(FIXTURE_DIR, spec, "file_only"))
        is_break.append(spec.is_break)
        names.append(spec.name)
        categories.append(spec.category)
    return records, is_break, names, categories


def _model_a_files(click_dir: Path) -> list[Path]:
    """All click/*.py files except those mirrored as fixture controls."""
    manifest = json.loads(MANIFEST_PATH.read_text())
    control_basenames = {
        Path(f["file"]).name
        for f in manifest["fixtures"]
        if not f["is_break"]
    }
    src = click_dir / "src" / "click"
    return [p for p in sorted(src.glob("*.py")) if p.name not in control_basenames]


def run(*, click_dir: Path, out: Path) -> dict[str, Any]:
    records, is_break, names, categories = _load_fixtures()
    model_a = _model_a_files(click_dir)
    if len(model_a) < 5:
        raise RuntimeError(
            f"Too few model_A files at {click_dir}/src/click ({len(model_a)}); "
            "expected click repo cloned at tag 8.1.7"
        )

    scorer = ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation="max")
    scorer.fit([], model_a_files=model_a)
    scores = scorer.score(records)

    break_scores = [s for s, b in zip(scores, is_break) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break) if not b]
    auc = auc_from_scores(break_scores, ctrl_scores)

    _write_report(out, auc, scores, is_break, names, categories, model_a)
    return {
        "auc": auc,
        "scores": scores,
        "names": names,
        "is_break": is_break,
        "model_a_count": len(model_a),
    }


def _verdict(auc: float) -> str:
    if auc >= 0.80:
        return f"**PASS** (AUC {auc:.4f} ≥ 0.80). Method generalises; proceed to Stage 4."
    if auc >= 0.65:
        return (
            f"**MIXED** (AUC {auc:.4f} in 0.65–0.80). Investigate per-category "
            "before any promotion decision."
        )
    return (
        f"**FAIL** (AUC {auc:.4f} < 0.65). Method is FastAPI-tuned; do not promote."
    )


def _write_report(
    out: Path,
    auc: float,
    scores: list[float],
    is_break: list[bool],
    names: list[str],
    categories: list[str],
    model_a: list[Path],
) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 Stage 3 — Tier 3 Cross-Domain Validation (click)\n",
        "Scorer: `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max')`",
        "",
        f"model_A: {len(model_a)} click source files "
        "(click/*.py minus 10 held-out controls)",
        "",
        f"**Overall AUC: {auc:.4f}**",
        "",
        "## Per-category scores",
        "",
        "| fixture | category | is_break | score |",
        "|---|---|---|---|",
    ]
    for n, c, b, s in zip(names, categories, is_break, scores):
        lines.append(f"| {n} | {c} | {b} | {s:.4f} |")
    lines += [
        "",
        "## Verdict",
        "",
        _verdict(auc),
        "",
    ]
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--click-dir", required=True, type=Path)
    parser.add_argument(
        "--out", type=Path,
        default=Path("docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md"),
    )
    args = parser.parse_args()
    result = run(click_dir=args.click_dir, out=args.out)
    print(f"Overall AUC: {result['auc']:.4f}", flush=True)


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run the end-to-end test**

Ensure click is cloned first:
```bash
[ -d /tmp/click-clone ] || git clone https://github.com/pallets/click /tmp/click-clone
git -C /tmp/click-clone checkout 8.1.7
```

Then:
```bash
uv run --package argot-engine pytest engine/argot/research/signal/phase13/test_validate_tier3_click.py -v
```
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/research/signal/phase13/validate_tier3_click.py \
        engine/argot/research/signal/phase13/test_validate_tier3_click.py
git commit -m "research(phase-13-s3): tier3 click validation runner"
```

---

### Task 5: Run end-to-end, inspect, write verdict

**Files:**
- Create: `docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md` (auto-generated, then hand-edited for verdict paragraph)

- [ ] **Step 1: Run the full validation**

```bash
cd /Users/damienmeur/projects/argot
uv run --package argot-engine python \
  engine/argot/research/signal/phase13/validate_tier3_click.py \
  --click-dir /tmp/click-clone \
  --out docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md
```

- [ ] **Step 2: Inspect the report + per-fixture scores**

Read `docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md`. Note:
- Overall AUC vs pre-registered gate
- Which category (argparse_class / optparse / raw_argv / docopt) scored highest, which lowest
- Any control fixture that scored as high as the break median — that's a fixture-design leak, not a scorer failure
- Any break that scored near zero — likely hunk range is wrong

- [ ] **Step 3: Append analysis section to the report (by hand)**

Add a `## Analysis` section under the auto-generated `## Verdict`:

- Top 3 highest-scoring breaks — were the anomalous treelets what you'd expect?
- Any misclassified fixtures (control scoring like a break, or vice versa) — diagnose
- Whether the AUC clears the pre-registered gate; if not, explicitly state the gate was missed (no moving the goalposts)
- A one-line recommendation: proceed to Stage 4? Re-design fixtures? Abandon the method?

Do NOT edit the auto-generated Verdict paragraph — it's a deterministic read of the AUC. The Analysis section is where interpretation lives.

- [ ] **Step 4: Verify `just verify` passes**

```bash
just verify
```
Expected: PASS. This is a research-only change so lint/type/test checks should all be green.

If `just verify` fails, diagnose and fix the root cause before committing (per repo CLAUDE.md — no broad suppressions).

- [ ] **Step 5: Commit the report**

```bash
git add docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md
git commit -m "research(phase-13-s3): tier3 click validation report (AUC=<value>)"
```

Replace `<value>` with the actual AUC from the report.

---

## Self-Review Checklist

- All 18 fixtures referenced in the manifest exist on disk (Task 2 + Task 3 tests enforce).
- Runner's `_model_a_files` correctly excludes controls by basename (Task 4).
- No FastAPI-specific imports or constants in any new file (verify via grep: `rg fastapi engine/argot/research/signal/phase13/validate_tier3_click.py` → no hits).
- Victory gate is pre-registered in this plan **and** in the runner's `_verdict()` function — the verdict text is deterministic from AUC, not hand-picked after seeing the result.
- No production scorer wiring changes: `ast_contrastive.py` is untouched.
- All tests in `test_validate_tier3_click.py` run under `uv run pytest`.

## Out of scope for this plan

- Real second catalog with domain-expert curation (post-validation, only if Tier 3 passes)
- TypeScript AST port via tree-sitter (Stage 4+)
- Parametrising the runner for repos other than click (hardcoded to click on purpose — this is a one-shot validation, not a library)
- Any changes to `ContrastiveAstTreeletScorer` itself — if the scorer is wrong, that's a separate debugging investigation, not a plan change
