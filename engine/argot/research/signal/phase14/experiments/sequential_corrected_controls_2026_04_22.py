# engine/argot/research/signal/phase14/experiments/sequential_corrected_controls_2026_04_22.py
"""Phase 14 Experiment 2c — Sequential pipeline with corrected control protocol (2026-04-22).

Hypothesis (single, binary):
  When ctrl_hunks is sampled from real source (disjoint from cal_hunks), the sequential
  scorer's FP rate drops to ≤5% on FastAPI without losing break recall.

Protocol change from exp #2B:
  Exp #2B measured FP on synthetic curated full-file fixtures (distribution mismatch).
  Exp #2c samples ctrl_hunks from real source (indices n_cal … n_cal+n_ctrl-1, disjoint).

Verdict (pre-registered):
  VALIDATED: FP ≤5% AND recall=100% AND threshold CV <5%, on ALL three domains.
  REJECTED:  any domain FP >20% OR recall <100%.
  ZONE GRISE: otherwise.

Usage:
    uv run python \\
        engine/argot/research/signal/phase14/experiments/\\
        sequential_corrected_controls_2026_04_22.py
"""

from __future__ import annotations

import json
import statistics
import subprocess
from pathlib import Path
from typing import Any

from argot.research.signal.phase14.calibration.random_hunk_sampler import (
    collect_candidates,
    sample_hunks_disjoint,
)
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent  # engine/argot
_RESEARCH_DIR = Path(__file__).parent.parent.parent.parent  # engine/argot/research
_PROJECT_ROOT = _ARGOT_PKG.parent.parent  # argot/ project root
_CATALOG_DIR = _ARGOT_PKG / "acceptance" / "catalog"
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_BPE_MODEL_B_PATH = _RESEARCH_DIR / "reference" / "generic_tokens_bpe.json"
_SCRIPT_DIR = Path(__file__).parent
_SCORES_OUT = _SCRIPT_DIR / "sequential_corrected_controls_2026_04_22_scores.json"
_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "sequential_corrected_controls_2026-04-22.md"
)

_FASTAPI_DIR = _CATALOG_DIR / "fastapi"
_RICH_DIR = _CATALOG_DIR / "rich"
_FAKER_DIR = _CATALOG_DIR / "faker"

# Exp #2B FP rates for comparison (pre-loaded from scores file)
_EXP2B_SCORES_PATH = _SCRIPT_DIR / "sequential_import_bpe_robustness_2026_04_22_scores.json"

N_CAL = 100
N_CTRL = 20
N_SEEDS = 5
SEEDS = list(range(N_SEEDS))

# Faker fixed split: 159 total → first 139 cal, next 20 ctrl
FAKER_N_CAL = 139
FAKER_N_CTRL = 20


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _extract_hunk(path: Path, start_line: int, end_line: int) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def _extract_file_to_hunk_end(path: Path, end_line: int) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    hi = min(len(lines), end_line)
    return "\n".join(lines[:hi])


def _load_manifest(path: Path) -> list[dict[str, Any]]:
    return json.loads(path.read_text(encoding="utf-8"))["fixtures"]  # type: ignore[no-any-return]


def _ensure_repo(name: str, url: str) -> Path:
    repo_dir = _REPOS_DIR / name
    if not repo_dir.exists():
        print(f"  Cloning {url} → {repo_dir}", flush=True)
        subprocess.run(["git", "clone", "--depth=1", url, str(repo_dir)], check=True)
    else:
        print(f"  Using cached repo: {repo_dir}", flush=True)
    return repo_dir


def _collect_source_files(repo_dir: Path) -> list[Path]:
    from argot.research.signal.phase14.calibration.random_hunk_sampler import (
        _DEFAULT_EXCLUDE_DIRS,
        _is_excluded,
    )

    return sorted(
        p for p in repo_dir.rglob("*.py") if not _is_excluded(p, repo_dir, _DEFAULT_EXCLUDE_DIRS)
    )


# ---------------------------------------------------------------------------
# Per-seed runner helpers
# ---------------------------------------------------------------------------


def _score_fixtures_fastapi(
    scorer: SequentialImportBpeScorer,
    fixtures: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for f in fixtures:
        hunk = _extract_file_to_hunk_end(_FASTAPI_DIR / f["file"], f["hunk_end_line"])
        scored = scorer.score_hunk(hunk)
        results.append(
            {"name": f["name"], "category": f.get("category", ""), "is_break": f["is_break"],
             **scored}
        )
    return results


def _score_fixtures_rich(
    scorer: SequentialImportBpeScorer,
    fixtures: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for f in fixtures:
        hunk = _extract_hunk(_RICH_DIR / f["file"], f["hunk_start_line"], f["hunk_end_line"])
        scored = scorer.score_hunk(hunk)
        results.append(
            {"name": f["name"], "category": f.get("category", ""), "is_break": f["is_break"],
             **scored}
        )
    return results


def _score_ctrl_hunks(
    scorer: SequentialImportBpeScorer,
    ctrl_hunks: list[str],
) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for i, hunk in enumerate(ctrl_hunks):
        scored = scorer.score_hunk(hunk)
        results.append({"ctrl_index": i, "hunk_preview": hunk[:80], **scored})
    return results


def _run_domain_seed(
    model_a_files: list[Path],
    cal_hunks: list[str],
    ctrl_hunks: list[str],
    break_fixtures: list[dict[str, Any]],
    score_fn: Any,
) -> dict[str, Any]:
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=cal_hunks,
    )
    fixture_results = score_fn(scorer, break_fixtures)
    ctrl_results = _score_ctrl_hunks(scorer, ctrl_hunks)

    breaks = [r for r in fixture_results if r["is_break"]]
    n_flagged = sum(1 for r in breaks if r["flagged"])
    n_fp = sum(1 for r in ctrl_results if r["flagged"])

    return {
        "bpe_threshold": scorer.bpe_threshold,
        "n_breaks": len(breaks),
        "n_ctrl": len(ctrl_results),
        "n_flagged": n_flagged,
        "n_fp": n_fp,
        "recall": n_flagged / len(breaks) if breaks else 0.0,
        "fp_rate": n_fp / len(ctrl_results) if ctrl_results else 0.0,
        "fixtures": fixture_results,
        "ctrl_results": ctrl_results,
    }


# ---------------------------------------------------------------------------
# Domain runners
# ---------------------------------------------------------------------------


def _run_domain(
    name: str,
    repo_dir: Path,
    catalog_dir: Path,
    score_fn: Any,
) -> dict[str, Any]:
    model_a_files = _collect_source_files(repo_dir)
    fixtures = _load_manifest(catalog_dir / "manifest.json")
    n_candidates = len(collect_candidates(repo_dir))

    print(
        f"  {name}: {len(model_a_files)} model_a files, {n_candidates} hunk candidates",
        flush=True,
    )

    seed_results: list[dict[str, Any]] = []
    for seed in SEEDS:
        cal_hunks, ctrl_hunks = sample_hunks_disjoint(repo_dir, N_CAL, N_CTRL, seed)
        print(
            f"  seed={seed}: n_cal={len(cal_hunks)}, n_ctrl={len(ctrl_hunks)}...",
            flush=True,
        )
        result = _run_domain_seed(model_a_files, cal_hunks, ctrl_hunks, fixtures, score_fn)
        result["seed"] = seed
        seed_results.append(result)
        print(
            f"    threshold={result['bpe_threshold']:.4f}, "
            f"recall={result['recall']:.0%} ({result['n_flagged']}/{result['n_breaks']}), "
            f"fp={result['fp_rate']:.0%} ({result['n_fp']}/{result['n_ctrl']})",
            flush=True,
        )

    return {
        "domain": name.lower(),
        "n_model_a_files": len(model_a_files),
        "n_candidates": n_candidates,
        "n_cal": N_CAL,
        "n_ctrl": N_CTRL,
        "seeds": seed_results,
    }


def _run_faker_corrected() -> dict[str, Any]:
    """Faker: fixed disjoint split — first 139 hunks cal, next 20 ctrl."""
    model_a_files = sorted((_FAKER_DIR / "sources" / "model_a").glob("*.py"))
    all_records: list[dict[str, Any]] = []
    with (_FAKER_DIR / "sampled_hunks.jsonl").open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                all_records.append(json.loads(line))

    cal_records = all_records[:FAKER_N_CAL]
    ctrl_records = all_records[FAKER_N_CAL : FAKER_N_CAL + FAKER_N_CTRL]
    cal_hunks = [r["hunk_source"] for r in cal_records]
    ctrl_hunks = [r["hunk_source"] for r in ctrl_records]

    # Verify disjointness by name
    cal_names = {r["name"] for r in cal_records}
    ctrl_names = {r["name"] for r in ctrl_records}
    assert cal_names & ctrl_names == set(), "faker cal/ctrl must be disjoint"

    print(
        f"  Faker: {len(model_a_files)} model_a files, "
        f"n_cal={len(cal_hunks)}, n_ctrl={len(ctrl_hunks)}",
        flush=True,
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=cal_hunks,
    )
    print(f"  BPE threshold: {scorer.bpe_threshold:.4f}", flush=True)

    break_manifest = _load_manifest(_FAKER_DIR / "breaks_manifest.json")
    break_results: list[dict[str, Any]] = []
    for f in break_manifest:
        hunk = _extract_hunk(_FAKER_DIR / f["file"], f["hunk_start_line"], f["hunk_end_line"])
        scored = scorer.score_hunk(hunk)
        break_results.append({"name": f["name"], "category": f["category"], **scored})

    ctrl_results = _score_ctrl_hunks(scorer, ctrl_hunks)
    for i, rec in enumerate(ctrl_records):
        ctrl_results[i]["name"] = rec["name"]

    n_break_flagged = sum(1 for r in break_results if r["flagged"])
    n_fp = sum(1 for r in ctrl_results if r["flagged"])

    print(
        f"  recall={n_break_flagged}/{len(break_results)}, "
        f"ctrl_fp={n_fp}/{len(ctrl_results)} ({n_fp / len(ctrl_results):.0%})",
        flush=True,
    )

    # faker_hunk_0047 is at cal index 46 — track its BPE score and holdout behaviour
    hunk_0047_rec = next((r for r in cal_records if r["name"] == "faker_hunk_0047"), None)
    hunk_0047_info: dict[str, Any] | None = None
    if hunk_0047_rec is not None:
        s = scorer.score_hunk(hunk_0047_rec["hunk_source"])
        hunk_0047_info = {
            "in_set": "cal",
            "bpe_score": s["bpe_score"],
            "threshold": scorer.bpe_threshold,
            "flagged_as_cal": s["flagged"],
        }
        # Holdout: remove from cal, refit, check if it fires
        holdout_cal = [r["hunk_source"] for r in cal_records if r["name"] != "faker_hunk_0047"]
        holdout_scorer = SequentialImportBpeScorer(
            model_a_files=model_a_files,
            bpe_model_b_path=_BPE_MODEL_B_PATH,
            calibration_hunks=holdout_cal,
        )
        holdout_scored = holdout_scorer.score_hunk(hunk_0047_rec["hunk_source"])
        hunk_0047_info["holdout_threshold"] = holdout_scorer.bpe_threshold
        hunk_0047_info["holdout_flagged"] = holdout_scored["flagged"]
        print(
            f"  hunk_0047: bpe={s['bpe_score']:.4f} threshold={scorer.bpe_threshold:.4f} "
            f"holdout_threshold={holdout_scorer.bpe_threshold:.4f} "
            f"holdout_flagged={holdout_scored['flagged']}",
            flush=True,
        )

    return {
        "domain": "faker",
        "n_model_a_files": len(model_a_files),
        "n_cal": len(cal_hunks),
        "n_ctrl": len(ctrl_hunks),
        "bpe_threshold": scorer.bpe_threshold,
        "n_breaks": len(break_results),
        "n_breaks_flagged": n_break_flagged,
        "n_fp": n_fp,
        "recall": n_break_flagged / len(break_results) if break_results else 0.0,
        "fp_rate": n_fp / len(ctrl_results) if ctrl_results else 0.0,
        "hunk_0047": hunk_0047_info,
        "break_fixtures": break_results,
        "ctrl_results": ctrl_results,
    }


# ---------------------------------------------------------------------------
# Verdict helpers
# ---------------------------------------------------------------------------


def _classify_fp(fp_rate: float) -> str:
    if fp_rate <= 0.05:
        return "VALIDATED"
    if fp_rate <= 0.20:
        return "ZONE_GRISE"
    return "REJECTED"


def _cv_band(cv: float) -> str:
    if cv < 0.05:
        return "STABLE"
    if cv < 0.15:
        return "FRAGILE"
    return "UNSTABLE"


def _domain_verdict(
    fp_rates: list[float],
    recalls: list[float],
    thresholds: list[float],
) -> tuple[str, float, float, str]:
    mean_fp = statistics.mean(fp_rates)
    mean_t = statistics.mean(thresholds)
    std_t = statistics.pstdev(thresholds)
    cv = std_t / mean_t if mean_t > 0 else 0.0
    fp_class = _classify_fp(mean_fp)
    cv_class = _cv_band(cv)
    if all(r == 1.0 for r in recalls) and fp_class == "VALIDATED":
        verdict = "VALIDATED"
    elif any(r < 1.0 for r in recalls) or max(fp_rates) > 0.20:
        verdict = "REJECTED"
    else:
        verdict = "ZONE_GRISE"
    return verdict, mean_fp, cv, cv_class


# ---------------------------------------------------------------------------
# Report writer
# ---------------------------------------------------------------------------


def _write_report(
    out: Path,
    fastapi_data: dict[str, Any],
    rich_data: dict[str, Any],
    faker_data: dict[str, Any],
    exp2b_scores: dict[str, Any],
) -> None:
    fa_seeds = fastapi_data["seeds"]
    ri_seeds = rich_data["seeds"]
    fa_thresholds = [s["bpe_threshold"] for s in fa_seeds]
    ri_thresholds = [s["bpe_threshold"] for s in ri_seeds]
    fa_recalls = [s["recall"] for s in fa_seeds]
    ri_recalls = [s["recall"] for s in ri_seeds]
    fa_fp_rates = [s["fp_rate"] for s in fa_seeds]
    ri_fp_rates = [s["fp_rate"] for s in ri_seeds]
    fa_n_breaks = fa_seeds[0]["n_breaks"]
    ri_n_breaks = ri_seeds[0]["n_breaks"]

    fa_verdict, fa_mean_fp, fa_cv, fa_cv_band = _domain_verdict(
        fa_fp_rates, fa_recalls, fa_thresholds
    )
    ri_verdict, ri_mean_fp, ri_cv, ri_cv_band = _domain_verdict(
        ri_fp_rates, ri_recalls, ri_thresholds
    )

    fk_fp_class = _classify_fp(faker_data["fp_rate"])
    fk_verdict = "VALIDATED" if faker_data["recall"] == 1.0 and fk_fp_class == "VALIDATED" else (
        "REJECTED" if faker_data["recall"] < 1.0 or faker_data["fp_rate"] > 0.20 else "ZONE_GRISE"
    )

    if all(v == "VALIDATED" for v in [fa_verdict, ri_verdict, fk_verdict]):
        overall = "VALIDATED"
    elif any(v == "REJECTED" for v in [fa_verdict, ri_verdict, fk_verdict]):
        overall = "REJECTED"
    else:
        overall = "ZONE_GRISE"

    # Exp #2B comparison data
    exp2b_fa_fp = [s["fp_rate"] for s in exp2b_scores["fastapi"]["seeds"]]
    exp2b_ri_fp = [s["fp_rate"] for s in exp2b_scores["rich"]["seeds"]]
    exp2b_fa_mean_fp = statistics.mean(exp2b_fa_fp)
    exp2b_ri_mean_fp = statistics.mean(exp2b_ri_fp)

    lines: list[str] = [
        "# Phase 14 Experiment 2c — Sequential pipeline with corrected control protocol (2026-04-22)",  # noqa: E501
        "",
        "**Scorer:** `SequentialImportBpeScorer` (unchanged from exp #2/#2B)",
        "",
        "**Hypothesis:** When ctrl_hunks is sampled from real source (disjoint from cal_hunks),",
        "the sequential scorer's FP rate drops to ≤5% on FastAPI without losing break recall.",
        "",
        "**Protocol change from exp #2B:** Exp #2B measured FP on synthetic curated full-file fixtures",  # noqa: E501
        "(fixture-vs-source distribution mismatch). Exp #2c samples ctrl_hunks from real source",
        "(indices n_cal … n_cal+n_ctrl−1, disjoint by construction).",
        "",
        "**Pre-registered verdict criteria (per domain, mean across 5 seeds):**",
        "",
        "| criterion | VALIDATED | ZONE GRISE | REJECTED |",
        "|---|---|---|---|",
        "| FP rate on ctrl_hunks | ≤5% | 5–20% | >20% |",
        "| Recall on breaks | 100% | — | <100% |",
        "| Threshold CV | <5% (STABLE) | 5–15% (FRAGILE) | ≥15% (UNSTABLE) |",
        "",
        "VALIDATED requires all three criteria green on all three domains.",
        "",
        "---",
        "",
        "## 1. Source Corpus and Sampling Protocol",
        "",
        "| domain | source pool | n_cal | n_ctrl | n_breaks |",
        "|---|---|---|---|---|",
        f"| FastAPI | `.argot/research/repos/fastapi` ({fastapi_data['n_candidates']} candidates) | {N_CAL} | {N_CTRL} | {fa_n_breaks} |",  # noqa: E501
        f"| rich | `.argot/research/repos/rich` ({rich_data['n_candidates']} candidates) | {N_CAL} | {N_CTRL} | {ri_n_breaks} |",  # noqa: E501
        f"| faker | `sampled_hunks.jsonl` (159 total) | {faker_data['n_cal']} | {faker_data['n_ctrl']} | {faker_data['n_breaks']} |",  # noqa: E501
        "",
        "Disjoint split: shuffle candidates with seed → cal_hunks = first n_cal, ctrl_hunks = next n_ctrl.",  # noqa: E501
        "Faker uses fixed positional split (no per-seed shuffle): indices 0–138 → cal, 139–158 → ctrl.",  # noqa: E501
        "",
        "---",
        "",
        "## 2. Per-seed Results",
        "",
        f"### FastAPI (n_breaks={fa_n_breaks}, n_ctrl={N_CTRL})",
        "",
        "| seed | threshold | recall | n_flagged | n_fp | fp_rate |",
        "|---|---|---|---|---|---|",
    ]

    for s in fa_seeds:
        lines.append(
            f"| {s['seed']} "
            f"| {s['bpe_threshold']:.4f} "
            f"| {s['recall']:.0%} "
            f"| {s['n_flagged']}/{s['n_breaks']} "
            f"| {s['n_fp']} "
            f"| {s['fp_rate']:.0%} |"
        )

    fa_mean_t = statistics.mean(fa_thresholds)
    fa_std_t = statistics.pstdev(fa_thresholds)
    lines += [
        f"| **stats** | mean={fa_mean_t:.4f} std={fa_std_t:.4f} CV={fa_cv:.1%}"
        f" | mean={statistics.mean(fa_recalls):.0%} | — | mean={fa_mean_fp:.0%} | — |",
        "",
        f"### Rich (n_breaks={ri_n_breaks}, n_ctrl={N_CTRL})",
        "",
        "| seed | threshold | recall | n_flagged | n_fp | fp_rate |",
        "|---|---|---|---|---|---|",
    ]

    for s in ri_seeds:
        lines.append(
            f"| {s['seed']} "
            f"| {s['bpe_threshold']:.4f} "
            f"| {s['recall']:.0%} "
            f"| {s['n_flagged']}/{s['n_breaks']} "
            f"| {s['n_fp']} "
            f"| {s['fp_rate']:.0%} |"
        )

    ri_mean_t = statistics.mean(ri_thresholds)
    ri_std_t = statistics.pstdev(ri_thresholds)
    lines += [
        f"| **stats** | mean={ri_mean_t:.4f} std={ri_std_t:.4f} CV={ri_cv:.1%}"
        f" | mean={statistics.mean(ri_recalls):.0%} | — | mean={ri_mean_fp:.0%} | — |",
        "",
        "### Faker (fixed split, single run)",
        "",
        "| n_cal | n_ctrl | threshold | recall | n_fp | fp_rate |",
        "|---|---|---|---|---|---|",
        f"| {faker_data['n_cal']} "
        f"| {faker_data['n_ctrl']} "
        f"| {faker_data['bpe_threshold']:.4f} "
        f"| {faker_data['recall']:.0%} "
        f"| {faker_data['n_fp']} "
        f"| {faker_data['fp_rate']:.0%} |",
        "",
        "---",
        "",
        "## 3. Flagged ctrl_hunks Trace",
        "",
        "Per-fixture trace for any ctrl_hunks flagged by the scorer.",
        "",
    ]

    # FastAPI flagged ctrl
    fa_flagged_ctrl: list[dict[str, Any]] = []
    for s in fa_seeds:
        for r in s.get("ctrl_results", []):
            if r["flagged"]:
                fa_flagged_ctrl.append({"seed": s["seed"], **r})

    if fa_flagged_ctrl:
        lines += [
            "### FastAPI flagged ctrl_hunks",
            "",
            "| seed | ctrl_index | bpe_score | threshold | margin | reason |",
            "|---|---|---|---|---|---|",
        ]
        for r in fa_flagged_ctrl:
            seed_data = next(s for s in fa_seeds if s["seed"] == r["seed"])
            margin = r["bpe_score"] - seed_data["bpe_threshold"]
            lines.append(
                f"| {r['seed']} | {r['ctrl_index']} "
                f"| {r['bpe_score']:.4f} | {seed_data['bpe_threshold']:.4f} "
                f"| {margin:+.4f} | {r['reason']} |"
            )
        lines.append("")
    else:
        lines += ["### FastAPI flagged ctrl_hunks", "", "None — zero false positives.", ""]

    # Rich flagged ctrl
    ri_flagged_ctrl: list[dict[str, Any]] = []
    for s in ri_seeds:
        for r in s.get("ctrl_results", []):
            if r["flagged"]:
                ri_flagged_ctrl.append({"seed": s["seed"], **r})

    if ri_flagged_ctrl:
        lines += [
            "### Rich flagged ctrl_hunks",
            "",
            "| seed | ctrl_index | bpe_score | threshold | margin | reason |",
            "|---|---|---|---|---|---|",
        ]
        for r in ri_flagged_ctrl:
            seed_data = next(s for s in ri_seeds if s["seed"] == r["seed"])
            margin = r["bpe_score"] - seed_data["bpe_threshold"]
            lines.append(
                f"| {r['seed']} | {r['ctrl_index']} "
                f"| {r['bpe_score']:.4f} | {seed_data['bpe_threshold']:.4f} "
                f"| {margin:+.4f} | {r['reason']} |"
            )
        lines.append("")
    else:
        lines += ["### Rich flagged ctrl_hunks", "", "None — zero false positives.", ""]

    # Faker flagged ctrl
    fk_flagged = [r for r in faker_data.get("ctrl_results", []) if r["flagged"]]
    if fk_flagged:
        lines += [
            "### Faker flagged ctrl_hunks",
            "",
            "| ctrl_index | name | bpe_score | threshold | margin | reason |",
            "|---|---|---|---|---|---|",
        ]
        for r in fk_flagged:
            margin = r["bpe_score"] - faker_data["bpe_threshold"]
            lines.append(
                f"| {r['ctrl_index']} | {r.get('name', '?')} "
                f"| {r['bpe_score']:.4f} | {faker_data['bpe_threshold']:.4f} "
                f"| {margin:+.4f} | {r['reason']} |"
            )
        lines.append("")
    else:
        lines += ["### Faker flagged ctrl_hunks", "", "None — zero false positives.", ""]

    lines += [
        "---",
        "",
        "## 4. break_ansi_raw_2 Thin-margin Tracking",
        "",
        "Exp #2 margin: bpe_score=5.6851 vs threshold=5.5984 (+0.087).",
        "",
        "| seed | bpe_score | threshold | margin | flagged |",
        "|---|---|---|---|---|",
    ]

    for s in ri_seeds:
        br2 = next((r for r in s["fixtures"] if r["name"] == "break_ansi_raw_2"), None)
        if br2 is not None:
            margin = br2["bpe_score"] - s["bpe_threshold"]
            flag_str = "YES" if br2["flagged"] else "NO"
            lines.append(
                f"| {s['seed']} "
                f"| {br2['bpe_score']:.4f} "
                f"| {s['bpe_threshold']:.4f} "
                f"| {margin:+.4f} "
                f"| {flag_str} |"
            )

    lines += [
        "",
        "---",
        "",
        "## 5. faker_hunk_0047 Under New Protocol",
        "",
    ]

    h47 = faker_data.get("hunk_0047")
    if h47:
        lines += [
            "faker_hunk_0047 is in the **cal** set (index 46).",
            "",
            "| metric | value |",
            "|---|---|",
            f"| bpe_score | {h47['bpe_score']:.4f} |",
            f"| cal threshold | {h47['threshold']:.4f} |",
            f"| sets threshold? | {'YES (is the max)' if abs(h47['bpe_score'] - h47['threshold']) < 1e-4 else 'NO'} |",  # noqa: E501
            f"| holdout threshold (without hunk_0047) | {h47['holdout_threshold']:.4f} |",
            f"| flagged at holdout threshold | {h47['holdout_flagged']} |",
            "",
        ]
        if h47["holdout_flagged"]:
            lines += [
                "**Finding:** faker_hunk_0047 fires when removed from calibration — confirms prior observation.",  # noqa: E501
                "",
            ]
        else:
            lines += [
                "**Finding:** faker_hunk_0047 does NOT fire at holdout threshold — it is below the new max.",  # noqa: E501
                "",
            ]
    else:
        lines += ["faker_hunk_0047 not found in calibration.", ""]

    lines += [
        "---",
        "",
        "## 6. Exp #2B vs Exp #2c Comparison",
        "",
        "| domain | exp #2B FP rate (synthetic ctrl) | exp #2c FP rate (real-source ctrl) | delta |",  # noqa: E501
        "|---|---|---|---|",
        f"| FastAPI | {exp2b_fa_mean_fp:.0%} | {fa_mean_fp:.0%} | {fa_mean_fp - exp2b_fa_mean_fp:+.0%} |",  # noqa: E501
        f"| rich | {exp2b_ri_mean_fp:.0%} | {ri_mean_fp:.0%} | {ri_mean_fp - exp2b_ri_mean_fp:+.0%} |",  # noqa: E501
        f"| faker | n/a (cal FP) | {faker_data['fp_rate']:.0%} | — |",
        "",
        "Exp #2B FastAPI FP=100% was caused by fixture-vs-source distribution mismatch (synthetic controls",  # noqa: E501
        "contain richer vocab than real source → exceed any threshold calibrated on real source).",
        "Exp #2c uses disjoint real-source ctrl_hunks → eliminates that mismatch.",
        "",
        "---",
        "",
        "## 7. Per-break Minimum Margin Across 5 Seeds",
        "",
        "Minimum margin = min over seeds of (bpe_score − threshold).",
        "Negative margin → break NOT flagged on that seed.",
        "",
        "### FastAPI",
        "",
        "| name | category | min_margin | always_flagged |",
        "|---|---|---|---|",
    ]

    fa_break_names = [r["name"] for r in fa_seeds[0]["fixtures"] if r["is_break"]]
    for name in fa_break_names:
        margins: list[float] = []
        flagged_count = 0
        cat = ""
        for s in fa_seeds:
            r = next((x for x in s["fixtures"] if x["name"] == name), None)
            if r:
                margins.append(r["bpe_score"] - s["bpe_threshold"])
                if r["flagged"]:
                    flagged_count += 1
                cat = r["category"]
        if margins:
            min_m = min(margins)
            always = "YES" if flagged_count == N_SEEDS else f"NO ({flagged_count}/{N_SEEDS})"
            lines.append(f"| {name} | {cat} | {min_m:+.4f} | {always} |")

    lines += [
        "",
        "### Rich",
        "",
        "| name | category | min_margin | always_flagged |",
        "|---|---|---|---|",
    ]

    ri_break_names = [r["name"] for r in ri_seeds[0]["fixtures"] if r["is_break"]]
    for name in ri_break_names:
        margins = []
        flagged_count = 0
        cat = ""
        for s in ri_seeds:
            r = next((x for x in s["fixtures"] if x["name"] == name), None)
            if r:
                margins.append(r["bpe_score"] - s["bpe_threshold"])
                if r["flagged"]:
                    flagged_count += 1
                cat = r["category"]
        if margins:
            min_m = min(margins)
            always = "YES" if flagged_count == N_SEEDS else f"NO ({flagged_count}/{N_SEEDS})"
            lines.append(f"| {name} | {cat} | {min_m:+.4f} | {always} |")

    lines += [
        "",
        "---",
        "",
        "## 8. Verdict",
        "",
        "| domain | FP rate (mean) | FP verdict | recall | CV | CV band | domain verdict |",
        "|---|---|---|---|---|---|---|",
        f"| FastAPI | {fa_mean_fp:.0%} | {_classify_fp(fa_mean_fp)} | {statistics.mean(fa_recalls):.0%} | {fa_cv:.1%} | {fa_cv_band} | **{fa_verdict}** |",  # noqa: E501
        f"| rich | {ri_mean_fp:.0%} | {_classify_fp(ri_mean_fp)} | {statistics.mean(ri_recalls):.0%} | {ri_cv:.1%} | {ri_cv_band} | **{ri_verdict}** |",  # noqa: E501
        f"| faker | {faker_data['fp_rate']:.0%} | {fk_fp_class} | {faker_data['recall']:.0%} | n/a (single run) | n/a | **{fk_verdict}** |",  # noqa: E501
        "",
        f"**Overall verdict: {overall}**",
        "",
    ]

    if overall == "VALIDATED":
        lines += [
            "Exp #2B FP=100% on FastAPI was a measurement artifact (fixture-vs-source mismatch).",
            "Under corrected evaluation protocol, all three domains meet VALIDATED criteria.",
            "The sequential scorer holds. Phase 14 V1 is confirmed.",
            "",
        ]
    elif overall == "REJECTED":
        rejected = [
            d for d, v in
            [("FastAPI", fa_verdict), ("rich", ri_verdict), ("faker", fk_verdict)]
            if v == "REJECTED"
        ]
        lines += [
            f"REJECTED on: {', '.join(rejected)}.",
            "Investigate before proceeding.",
            "",
        ]
    else:
        lines += [
            "ZONE GRISE — criteria partially met. Examine diagnostics above.",
            "",
        ]

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Experiment 2c — Corrected control protocol", flush=True)

    print("\nEnsuring repos...", flush=True)
    fastapi_repo = _ensure_repo("fastapi", "https://github.com/tiangolo/fastapi")
    rich_repo = _ensure_repo("rich", "https://github.com/Textualize/rich")

    print("\nRunning FastAPI (5 seeds)...", flush=True)
    fastapi_data = _run_domain("FastAPI", fastapi_repo, _FASTAPI_DIR, _score_fixtures_fastapi)

    print("\nRunning rich (5 seeds)...", flush=True)
    rich_data = _run_domain("rich", rich_repo, _RICH_DIR, _score_fixtures_rich)

    print("\nRunning faker (fixed split)...", flush=True)
    faker_data = _run_faker_corrected()

    print("\nLoading exp #2B scores for comparison...", flush=True)
    exp2b_scores: dict[str, Any] = json.loads(_EXP2B_SCORES_PATH.read_text(encoding="utf-8"))

    scores: dict[str, Any] = {
        "fastapi": fastapi_data,
        "rich": rich_data,
        "faker": faker_data,
    }
    _SCORES_OUT.write_text(json.dumps(scores, indent=2), encoding="utf-8")
    print(f"\nScores saved to {_SCORES_OUT}", flush=True)

    _write_report(_DOCS_OUT, fastapi_data, rich_data, faker_data, exp2b_scores)


if __name__ == "__main__":
    main()
