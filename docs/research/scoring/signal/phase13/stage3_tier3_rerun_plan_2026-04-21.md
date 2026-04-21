# Phase 13 Stage 3 — Tier 3 Methodology-Controlled Rerun (click)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rerun Tier 3 cross-domain validation on click with two structural fixes that the initial run conflated with "method doesn't generalise" — discriminate between (a) *method is FastAPI-tuned* and (b) *Tier 3 setup was structurally unfair*.

**Architecture:** Same scorer, same 8 break fixtures, same repo (click 8.1.7). Two changes:
1. **Shrink the held-out set from 10 files → 3 files.** The first run held out 10 of click's ~16 source files, leaving `model_A` with just 6 files. That is too small to represent click — click-idiomatic treelets absent from those 6 files land at `A[t]=0`, which makes pure-click controls look *maximally anomalous* under `max` aggregation. Holding out only `decorators.py`, `types.py`, `core.py` leaves `model_A` with 13 files (~4.7K LoC of click excluded, ~12K LoC retained).
2. **Size-match controls to breaks.** The first run used whole-file controls (68–3042 lines → thousands of treelets) vs. ~15-line break hunks (tens of treelets). `max` over many samples > `max` over few, even with zero signal. This rerun uses 10 × ~20-line hunks drawn from the 3 held-out files as controls, matching break hunk sizes.

If AUC recovers to ≥ 0.65 with these fixes, the initial "abandon" verdict was premature — the method is *corpus-size and hunk-size sensitive*, not FastAPI-tuned. If AUC still fails, the recommendation stands.

**Tech Stack:** Python 3.13, `uv`, `pytest`, existing `ContrastiveAstTreeletScorer`, existing `fixture_to_record` / `auc_from_scores` / `extract_treelets`. No new dependencies. New manifest and runner only — no new fixture content (control hunks are line ranges into existing `controls/*.py` files).

**Victory gate (pre-registered — same as original Tier 3 plan, no goalposts moved):**
| AUC | Interpretation |
|---|---|
| ≥ 0.80 | PASS — method generalises; proceed to Stage 4 |
| 0.65 – 0.80 | MIXED — investigate per-category before promotion |
| < 0.65 | FAIL — method is genuinely FastAPI-tuned; abandon per original recommendation |

**Interpretive addendum:** The rerun report must also explicitly compare the new AUC to the prior run's 0.1187 and name which of (a)/(b) the evidence supports.

---

## File Structure

**New files:**
- `engine/argot/research/signal/phase13/tier3_fixtures/click/manifest_matched.json` — new manifest: 10 hunk-sized controls + 8 existing breaks
- `engine/argot/research/signal/phase13/validate_tier3_click_matched.py` — new runner (copy of `validate_tier3_click.py` with two changes: loads `manifest_matched.json`, excludes a smaller held-out set)
- `engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py` — pytest tests for manifest + runner
- `docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md` — final report (auto-written verdict + hand-written analysis section)

**Modified files:** none. No production code changes; additive research files only.

**Reused (not modified):**
- Existing `breaks/*.py` (8 files) — referenced by path in new manifest
- Existing `controls/{decorators,types,core}.py` — referenced as hunk ranges (not copied)
- Existing `ContrastiveAstTreeletScorer` — untouched

**External prerequisite (unchanged):** `/tmp/click-clone` at tag `8.1.7`.

---

### Task 1: Write matched-size control hunk manifest

**Files:**
- Create: `engine/argot/research/signal/phase13/tier3_fixtures/click/manifest_matched.json`
- Create: `engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py`

We will define 10 control hunks of ~20 lines each, drawn from 3 held-out click files spanning decorator, type, and core surfaces. The hunk boundaries target a single self-contained construct (one function, one class body, one idiomatic block) so treelets come from coherent click idioms.

- [ ] **Step 1: Write the failing test**

Create `engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py`:

```python
"""Tests for Tier 3 methodology-controlled rerun on click."""
from __future__ import annotations

import ast
import json
from pathlib import Path

FIXTURE_DIR = Path(__file__).parent / "tier3_fixtures" / "click"
CONTROLS_DIR = FIXTURE_DIR / "controls"
BREAKS_DIR = FIXTURE_DIR / "breaks"
MANIFEST_PATH = FIXTURE_DIR / "manifest_matched.json"

HELD_OUT = {"decorators.py", "types.py", "core.py"}
CONTROL_TARGET_LINES = 20
CONTROL_TOLERANCE = 8  # hunks must be within 20 ± 8 lines (12–28)


def test_manifest_counts() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    controls = [f for f in manifest["fixtures"] if not f["is_break"]]
    breaks = [f for f in manifest["fixtures"] if f["is_break"]]
    assert len(controls) == 10, f"expected 10 controls, got {len(controls)}"
    assert len(breaks) == 8, f"expected 8 breaks, got {len(breaks)}"


def test_controls_only_reference_held_out_files() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if f["is_break"]:
            continue
        basename = Path(f["file"]).name
        assert basename in HELD_OUT, (
            f"control {f['name']} references {basename}, not in held-out set {HELD_OUT}"
        )


def test_control_hunks_are_size_matched() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if f["is_break"]:
            continue
        hunk_size = f["hunk_end_line"] - f["hunk_start_line"] + 1
        low = CONTROL_TARGET_LINES - CONTROL_TOLERANCE
        high = CONTROL_TARGET_LINES + CONTROL_TOLERANCE
        assert low <= hunk_size <= high, (
            f"control {f['name']} hunk size {hunk_size} outside [{low}, {high}]"
        )


def test_control_hunk_ranges_valid() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if f["is_break"]:
            continue
        path = FIXTURE_DIR / f["file"]
        line_count = len(path.read_text().splitlines())
        assert 1 <= f["hunk_start_line"] <= f["hunk_end_line"] <= line_count, (
            f"invalid range in {f['name']}: "
            f"{f['hunk_start_line']}-{f['hunk_end_line']} for {line_count}-line file"
        )


def test_break_entries_reuse_existing_fixtures() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if not f["is_break"]:
            continue
        path = FIXTURE_DIR / f["file"]
        assert path.exists(), f"break fixture missing: {path}"
        ast.parse(path.read_text())
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd /Users/damienmeur/projects/argot
uv run --package argot-engine pytest \
    engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py -v
```
Expected: all 5 tests FAIL (manifest does not exist).

- [ ] **Step 3: Select the 10 control hunk ranges**

Open each held-out file and pick contiguous ~20-line ranges that bracket *one* self-contained construct per hunk. Use `sed -n 'START,ENDp' /path/to/file` to eyeball each range before committing.

Target hunks (all line numbers are from click 8.1.7):

**From `decorators.py` (561 lines — pick 3 hunks):**
- `decorators_pass_context` — the `@pass_context` decorator implementation: lines 26–45 (20 lines)
- `decorators_command` — the body of `def command(...)`: lines 146–167 (22 lines)
- `decorators_option` — the body of `def option(...)`: lines 329–351 (23 lines)

**From `types.py` (1089 lines — pick 4 hunks):**
- `types_int_range_body` — `IntRange.__init__` + conversion: lines 338–358 (21 lines)
- `types_choice_convert` — `Choice.convert`: lines 242–263 (22 lines)
- `types_path_convert` — `Path.convert`: lines 776–797 (22 lines)
- `types_paramtype_convert` — `ParamType.convert` + helpers: lines 95–114 (20 lines)

**From `core.py` (3042 lines — pick 3 hunks):**
- `core_context_init_tail` — tail of `Context.__init__`: lines 228–249 (22 lines)
- `core_command_parse_args` — `Command.parse_args`: lines 1420–1441 (22 lines)
- `core_group_add_command` — `Group.add_command` + `Group.command`: lines 1720–1742 (23 lines)

If any range is off (e.g., click 8.1.7 renamed a method), adjust to the nearest self-contained 18–24-line block and document the substitution in the manifest's `rationale`.

- [ ] **Step 4: Write `manifest_matched.json`**

Write the new manifest with 10 control entries (hunk ranges pointing into `controls/decorators.py`, `controls/types.py`, `controls/core.py`) plus the 8 existing break entries copied verbatim from `manifest.json`.

```json
{
  "source_repo": "https://github.com/pallets/click",
  "source_tag": "8.1.7",
  "note": "Methodology-controlled rerun — 3 held-out files, size-matched ~20-line control hunks. See stage3_tier3_rerun_plan_2026-04-21.md.",
  "fixtures": [
    {
      "name": "control_decorators_pass_context",
      "file": "controls/decorators.py",
      "hunk_start_line": 26,
      "hunk_end_line": 45,
      "is_break": false,
      "rationale": "pass_context decorator body — canonical click decorator idiom",
      "category": "control_decorators"
    },
    {
      "name": "control_decorators_command",
      "file": "controls/decorators.py",
      "hunk_start_line": 146,
      "hunk_end_line": 167,
      "is_break": false,
      "rationale": "command() factory decorator body",
      "category": "control_decorators"
    },
    {
      "name": "control_decorators_option",
      "file": "controls/decorators.py",
      "hunk_start_line": 329,
      "hunk_end_line": 351,
      "is_break": false,
      "rationale": "option() factory decorator body",
      "category": "control_decorators"
    },
    {
      "name": "control_types_int_range",
      "file": "controls/types.py",
      "hunk_start_line": 338,
      "hunk_end_line": 358,
      "is_break": false,
      "rationale": "IntRange type conversion — canonical click ParamType subclass",
      "category": "control_types"
    },
    {
      "name": "control_types_choice_convert",
      "file": "controls/types.py",
      "hunk_start_line": 242,
      "hunk_end_line": 263,
      "is_break": false,
      "rationale": "Choice.convert — canonical ParamType.convert implementation",
      "category": "control_types"
    },
    {
      "name": "control_types_path_convert",
      "file": "controls/types.py",
      "hunk_start_line": 776,
      "hunk_end_line": 797,
      "is_break": false,
      "rationale": "Path.convert — file-path ParamType idioms",
      "category": "control_types"
    },
    {
      "name": "control_types_paramtype_convert",
      "file": "controls/types.py",
      "hunk_start_line": 95,
      "hunk_end_line": 114,
      "is_break": false,
      "rationale": "ParamType.convert base — abstract type interface",
      "category": "control_types"
    },
    {
      "name": "control_core_context_init_tail",
      "file": "controls/core.py",
      "hunk_start_line": 228,
      "hunk_end_line": 249,
      "is_break": false,
      "rationale": "tail of Context.__init__ — click Context setup idioms",
      "category": "control_core"
    },
    {
      "name": "control_core_command_parse_args",
      "file": "controls/core.py",
      "hunk_start_line": 1420,
      "hunk_end_line": 1441,
      "is_break": false,
      "rationale": "Command.parse_args — canonical click parsing flow",
      "category": "control_core"
    },
    {
      "name": "control_core_group_add_command",
      "file": "controls/core.py",
      "hunk_start_line": 1720,
      "hunk_end_line": 1742,
      "is_break": false,
      "rationale": "Group.add_command + Group.command — composition idioms",
      "category": "control_core"
    },
    {
      "name": "break_argparse_class_based_1",
      "file": "breaks/break_argparse_class_based_1.py",
      "hunk_start_line": 13,
      "hunk_end_line": 27,
      "is_break": true,
      "rationale": "argparse class-based paradigm in click-looking file",
      "category": "argparse_class"
    },
    {
      "name": "break_argparse_class_based_2",
      "file": "breaks/break_argparse_class_based_2.py",
      "hunk_start_line": 13,
      "hunk_end_line": 27,
      "is_break": true,
      "rationale": "argparse class-based paradigm in click-looking file",
      "category": "argparse_class"
    },
    {
      "name": "break_argparse_class_based_3",
      "file": "breaks/break_argparse_class_based_3.py",
      "hunk_start_line": 13,
      "hunk_end_line": 28,
      "is_break": true,
      "rationale": "argparse class-based paradigm in click-looking file",
      "category": "argparse_class"
    },
    {
      "name": "break_optparse_deprecated_1",
      "file": "breaks/break_optparse_deprecated_1.py",
      "hunk_start_line": 13,
      "hunk_end_line": 27,
      "is_break": true,
      "rationale": "optparse deprecated paradigm in click-looking file",
      "category": "optparse"
    },
    {
      "name": "break_optparse_deprecated_2",
      "file": "breaks/break_optparse_deprecated_2.py",
      "hunk_start_line": 14,
      "hunk_end_line": 27,
      "is_break": true,
      "rationale": "optparse deprecated paradigm in click-looking file",
      "category": "optparse"
    },
    {
      "name": "break_raw_sys_argv_1",
      "file": "breaks/break_raw_sys_argv_1.py",
      "hunk_start_line": 13,
      "hunk_end_line": 31,
      "is_break": true,
      "rationale": "raw sys.argv manual parsing in click-looking file",
      "category": "raw_argv"
    },
    {
      "name": "break_raw_sys_argv_2",
      "file": "breaks/break_raw_sys_argv_2.py",
      "hunk_start_line": 14,
      "hunk_end_line": 33,
      "is_break": true,
      "rationale": "raw sys.argv manual parsing in click-looking file",
      "category": "raw_argv"
    },
    {
      "name": "break_docopt_style_1",
      "file": "breaks/break_docopt_style_1.py",
      "hunk_start_line": 13,
      "hunk_end_line": 37,
      "is_break": true,
      "rationale": "docopt-style manual parsing in click-looking file",
      "category": "docopt"
    }
  ]
}
```

- [ ] **Step 5: Spot-check each control hunk is a coherent construct**

For each control entry, print its content and confirm it brackets one self-contained block (not a half-function). Example:

```bash
sed -n '26,45p' engine/argot/research/signal/phase13/tier3_fixtures/click/controls/decorators.py
```

Expected: a complete `pass_context` decorator definition, no dangling `def` or `class` at the edges. Repeat for all 10. If a hunk lands mid-function because line numbers drift, adjust the `hunk_start_line`/`hunk_end_line` by ±3 lines to re-align with construct boundaries.

- [ ] **Step 6: Run the tests to verify they pass**

```bash
uv run --package argot-engine pytest \
    engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py -v
```
Expected: all 5 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add engine/argot/research/signal/phase13/tier3_fixtures/click/manifest_matched.json \
        engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py
git commit -m "research(phase-13-s3): matched-size control manifest for tier3 rerun"
```

---

### Task 2: Write the methodology-controlled runner

**Files:**
- Create: `engine/argot/research/signal/phase13/validate_tier3_click_matched.py`
- Modify: `engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py`

The runner is a near-copy of `validate_tier3_click.py` with two differences:
1. Loads `manifest_matched.json` (not `manifest.json`).
2. `_model_a_files` excludes a hardcoded set of 3 held-out filenames (not "all control basenames").

- [ ] **Step 1: Add the failing end-to-end test**

Append to `test_validate_tier3_click_matched.py`:

```python
def test_runner_produces_auc_and_model_a_count(tmp_path: Path) -> None:
    """End-to-end: load matched manifest, build model_A=13, score, write report."""
    import os
    click_dir = Path(os.environ.get("TIER3_CLICK_DIR", "/tmp/click-clone"))
    if not click_dir.exists():
        import pytest
        pytest.skip(f"click clone not found at {click_dir}; set TIER3_CLICK_DIR")

    from argot.research.signal.phase13.validate_tier3_click_matched import run

    out = tmp_path / "report.md"
    result = run(click_dir=click_dir, out=out)

    assert "auc" in result
    assert 0.0 <= result["auc"] <= 1.0
    # With 3 held-out files and ~16 click/*.py total, we expect exactly 13 model_A files.
    assert result["model_a_count"] == 13, (
        f"expected model_A=13 (16 click files minus 3 held-out), got {result['model_a_count']}"
    )
    assert out.exists()
    body = out.read_text()
    assert "Verdict" in body
    assert "Comparison to v1" in body
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
uv run --package argot-engine pytest \
    engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py::test_runner_produces_auc_and_model_a_count -v
```
Expected: FAIL — module does not exist.

- [ ] **Step 3: Write the runner**

Create `engine/argot/research/signal/phase13/validate_tier3_click_matched.py`:

```python
"""Phase 13 Stage 3 — Tier 3 methodology-controlled rerun on click.

Rerun with two structural fixes vs. validate_tier3_click.py:
  1. held-out set shrunk 10 → 3 (model_A grows 6 → 13 files)
  2. controls are ~20-line hunks, matched to break hunk sizes

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/validate_tier3_click_matched.py \\
        --click-dir /tmp/click-clone \\
        --out docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md
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
MANIFEST_PATH = FIXTURE_DIR / "manifest_matched.json"

# The 3 held-out click source filenames. Controls are hunks drawn from these.
HELD_OUT: frozenset[str] = frozenset({"decorators.py", "types.py", "core.py"})

# Prior-run AUC (see stage3_tier3_click_2026-04-21.md) — referenced in report comparison.
V1_AUC = 0.1187


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
    """All click/*.py files except the 3 held-out sources."""
    src = click_dir / "src" / "click"
    return [p for p in sorted(src.glob("*.py")) if p.name not in HELD_OUT]


def run(*, click_dir: Path, out: Path) -> dict[str, Any]:
    records, is_break, names, categories = _load_fixtures()
    model_a = _model_a_files(click_dir)
    if len(model_a) < 10:
        raise RuntimeError(
            f"Too few model_A files at {click_dir}/src/click ({len(model_a)}); "
            "expected click repo cloned at tag 8.1.7 (16 files, 13 after hold-out)"
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
        return (
            f"**PASS** (AUC {auc:.4f} ≥ 0.80). Method generalises once corpus and hunk "
            "sizes are matched; proceed to Stage 4."
        )
    if auc >= 0.65:
        return (
            f"**MIXED** (AUC {auc:.4f} in 0.65–0.80). Method shows signal after "
            "methodology fixes but not enough to promote — investigate per-category."
        )
    return (
        f"**FAIL** (AUC {auc:.4f} < 0.65). Method does not generalise to click even "
        "with corpus and hunk sizes matched; prior-run abandonment recommendation stands."
    )


def _comparison(auc: float) -> str:
    delta = auc - V1_AUC
    if auc >= 0.65:
        interp = (
            "Recovery above the FAIL threshold. The v1 result was an artifact of the "
            "6-file model_A and whole-file controls, not evidence that the scorer is "
            "FastAPI-tuned. Remediation (larger corpus / matched hunk sizes), not "
            "abandonment, was the correct response."
        )
    elif delta >= 0.30:
        interp = (
            "Partial recovery. Methodology fixes account for a large share of v1's "
            "failure, but the scorer still underperforms the gate — genuine "
            "cross-domain weakness, not purely an artifact."
        )
    else:
        interp = (
            "No meaningful recovery. Methodology fixes do not rescue the scorer — "
            "v1's 'FastAPI-tuned' verdict is supported by this rerun."
        )
    return (
        f"Prior run (v1, 6-file model_A, whole-file controls): **AUC {V1_AUC:.4f}**\n\n"
        f"This run (v2, 13-file model_A, size-matched controls): **AUC {auc:.4f}** "
        f"(Δ = {delta:+.4f})\n\n"
        f"{interp}"
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
        "# Phase 13 Stage 3 — Tier 3 Methodology-Controlled Rerun (click)\n",
        "Scorer: `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max')`",
        "",
        f"model_A: {len(model_a)} click source files "
        "(click/*.py minus 3 held-out files: decorators.py, types.py, core.py)",
        "",
        "Controls: 10 × ~20-line hunks drawn from the 3 held-out files "
        "(size-matched to break hunks).",
        "",
        f"**Overall AUC: {auc:.4f}**",
        "",
        "## Per-fixture scores",
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
        "## Comparison to v1",
        "",
        _comparison(auc),
        "",
    ]
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--click-dir", required=True, type=Path)
    parser.add_argument(
        "--out", type=Path,
        default=Path(
            "docs/research/scoring/signal/phase13/"
            "stage3_tier3_click_matched_2026-04-21.md"
        ),
    )
    args = parser.parse_args()
    result = run(click_dir=args.click_dir, out=args.out)
    print(f"Overall AUC: {result['auc']:.4f}", flush=True)


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Ensure click clone is available**

```bash
[ -d /tmp/click-clone ] || git clone https://github.com/pallets/click /tmp/click-clone
git -C /tmp/click-clone checkout 8.1.7
```

- [ ] **Step 5: Run the end-to-end test**

```bash
uv run --package argot-engine pytest \
    engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py -v
```
Expected: all tests PASS including `test_runner_produces_auc_and_model_a_count`.

- [ ] **Step 6: Commit**

```bash
git add engine/argot/research/signal/phase13/validate_tier3_click_matched.py \
        engine/argot/research/signal/phase13/test_validate_tier3_click_matched.py
git commit -m "research(phase-13-s3): tier3 rerun runner with matched methodology"
```

---

### Task 3: Run end-to-end, inspect, write analysis

**Files:**
- Create: `docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md` (auto-written, then analysis section appended)

- [ ] **Step 1: Run the full validation**

```bash
cd /Users/damienmeur/projects/argot
uv run --package argot-engine python \
    engine/argot/research/signal/phase13/validate_tier3_click_matched.py \
    --click-dir /tmp/click-clone \
    --out docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md
```

- [ ] **Step 2: Inspect the report + per-fixture scores**

Read the auto-written report. Note:
- Overall AUC vs. pre-registered gate
- Per-category break scores (`argparse_class`, `optparse`, `raw_argv`, `docopt`) — are any category consistently mid-pack vs. controls?
- Per-control scores — any single control scoring like a break (fixture-design leak) or scoring much higher than peers (treelet-mass outlier)
- Whether the v1 → v2 delta is concentrated in controls (now scoring lower) or breaks (now scoring higher), or both

- [ ] **Step 3: Append an Analysis section by hand**

Add a `## Analysis` section after the auto-written `## Comparison to v1`. Cover, in order:

1. **Which hypothesis the data supports.** One of:
   - "Setup was unfair" — AUC ≥ 0.65 → v1 verdict was premature; method is corpus-size-sensitive, not FastAPI-tuned.
   - "Partly unfair, partly weak" — AUC in [0.35, 0.65) and Δ ≥ 0.30 → methodology matters, but scorer still under-generalises.
   - "Genuinely FastAPI-tuned" — AUC still < 0.35 or Δ < 0.30 → v1's abandonment recommendation stands.
2. **Which break categories responded best/worst**, and whether the ordering is the same as FastAPI's break categories (flask, django_cbv, aiohttp, etc.) or fundamentally different.
3. **Whether any control scores are outliers** — if one ~20-line hunk scores far above peers, read its source; a leak here (e.g., the hunk happens to include `optparse`/`argparse` strings in docstrings) would distort the AUC.
4. **Next action.** One of:
   - Promote (AUC ≥ 0.80) — proceed to Stage 4
   - Investigate per-category, do NOT abandon (AUC 0.65–0.80)
   - Abandon contrastive approach, move to CodeBERT MLM surprise baseline per the `try simple literature baselines first` feedback (AUC < 0.65)

Do NOT edit the auto-written Verdict paragraph — it is deterministic from AUC. Analysis is the interpretive layer.

- [ ] **Step 4: Verify `just verify` passes**

```bash
just verify
```
Expected: PASS. Research-only change; lint/type/test checks should all be green. If it fails, diagnose the root cause per project CLAUDE.md — no broad suppressions.

- [ ] **Step 5: Commit the report**

```bash
git add docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md
git commit -m "research(phase-13-s3): tier3 rerun report (AUC=<value>, v1 Δ=<delta>)"
```

Replace `<value>` and `<delta>` with actual values from the report.

---

## Self-Review Checklist

- All 18 fixtures referenced in `manifest_matched.json` exist on disk (enforced by Task 1 tests).
- Control hunks are ~20 lines ± 8 and come only from held-out files (enforced by Task 1 tests).
- `_model_a_files` returns exactly 13 files when pointed at `/tmp/click-clone` 8.1.7 (enforced by Task 2 test).
- Victory gate is identical to original Tier 3 plan (0.80 / 0.65) — no goalposts moved.
- Report auto-generates a Verdict *and* a v1 comparison table — interpretation cannot rely on hand-edited prose alone.
- No changes to `ContrastiveAstTreeletScorer`, reference corpus, or any production code.
- No FastAPI-specific imports in new files: `rg fastapi engine/argot/research/signal/phase13/validate_tier3_click_matched.py` → no hits.

## Out of scope for this plan

- Running the same rerun on a second repo (e.g., `rich`, `httpx`) — if v2 passes on click, a second-repo confirmation becomes the next plan, not this one.
- Any change to the contrastive scorer itself — if v2 still fails, the follow-up is a *different scorer* (CodeBERT MLM surprise), not a patch to `ast_contrastive`.
- Sweeping aggregation (`mean` vs `max`) or `epsilon` — this rerun holds scorer hyperparameters exactly as v1 used them to isolate the methodology variable.
- Changing break fixtures — the 8 existing breaks are reused verbatim to make v1/v2 AUCs directly comparable.
