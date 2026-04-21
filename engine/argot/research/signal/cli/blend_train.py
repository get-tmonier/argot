"""Phase 12 S5 — Ensemble blend of top-3 scorers.

Reads the bakeoff JSON scores file, selects the top-3 scorers by overall AUC,
does a simplex grid search over blend weights, and writes a report + config JSON.

Usage::

    uv run --package argot-engine python -m argot.research.signal.cli.blend_train \\
        --scores docs/research/scoring/signal/phase12/b_scores_<date>.json \\
        --out docs/research/scoring/signal/phase12
"""

from __future__ import annotations

import argparse
import datetime
import json
import statistics
from pathlib import Path
from typing import Any

from argot.research.signal.bootstrap import auc_from_scores

# Phase 11 production winner AUC
_PHASE11_WINNER_AUC = 0.6532

# Victory gate for phase 12
_VICTORY_GATE = 0.80


# ---------------------------------------------------------------------------
# Simplex grid search helpers
# ---------------------------------------------------------------------------


def _simplex_points(n_scorers: int = 3, step: float = 0.05) -> list[tuple[float, ...]]:
    """Enumerate all convex combinations on the simplex with given step size.

    For n_scorers=3, step=0.05 this produces 231 points.
    """
    n_steps = int(round(1.0 / step))
    if n_scorers == 2:
        return [
            (i / n_steps, (n_steps - i) / n_steps)
            for i in range(n_steps + 1)
        ]
    if n_scorers == 3:
        return [
            (i / n_steps, j / n_steps, (n_steps - i - j) / n_steps)
            for i in range(n_steps + 1)
            for j in range(n_steps + 1 - i)
            if i + j <= n_steps
        ]
    raise ValueError(f"_simplex_points only supports n_scorers in {{2, 3}}, got {n_scorers}")


def _z_normalize(scores: list[float]) -> tuple[list[float], float, float]:
    """Return (z_scores, mean, std)."""
    mean = statistics.mean(scores) if scores else 0.0
    std = statistics.stdev(scores) if len(scores) >= 2 else 1.0
    if std == 0.0:
        std = 1.0
    z = [(s - mean) / std for s in scores]
    return z, mean, std


# ---------------------------------------------------------------------------
# Core logic
# ---------------------------------------------------------------------------


def _find_best_alpha(
    fixture_z_scores: dict[str, list[float]],
    fixture_is_break: list[bool],
    scorer_names: list[str],
) -> tuple[tuple[float, ...], float]:
    """Grid-search the simplex for the α that maximises blended AUC.

    Parameters
    ----------
    fixture_z_scores:
        Mapping scorer_name -> list of z-normalised scores, one per fixture
    fixture_is_break:
        Whether each fixture is a break (True) or control (False)
    scorer_names:
        The 3 (or 2) scorer names to blend

    Returns
    -------
    (best_alpha, best_auc)
    """
    points = _simplex_points(n_scorers=len(scorer_names))
    z_arrays = [fixture_z_scores[name] for name in scorer_names]
    n_fixtures = len(fixture_is_break)

    best_auc = -1.0
    best_alpha: tuple[float, ...] = points[0]

    for alphas in points:
        blended = [
            sum(alphas[k] * z_arrays[k][i] for k in range(len(scorer_names)))
            for i in range(n_fixtures)
        ]
        break_scores = [blended[i] for i in range(n_fixtures) if fixture_is_break[i]]
        ctrl_scores = [blended[i] for i in range(n_fixtures) if not fixture_is_break[i]]
        auc = auc_from_scores(break_scores, ctrl_scores)
        if auc > best_auc:
            best_auc = auc
            best_alpha = alphas

    return best_alpha, best_auc


def _run_blend_train(scores_path: Path, out_dir: Path) -> None:
    """Main logic: load JSON, find top-3, grid-search, write report + config."""
    with scores_path.open() as f:
        data: dict[str, Any] = json.load(f)

    scorer_names: list[str] = data["scorers"]
    fixtures: list[dict[str, Any]] = data["fixtures"]
    scorer_aucs: dict[str, dict[str, Any]] = data["scorer_aucs"]

    # ------------------------------------------------------------------
    # 1. Rank scorers by overall AUC, pick top-3
    # ------------------------------------------------------------------
    ranked = sorted(scorer_names, key=lambda n: scorer_aucs[n]["overall"], reverse=True)
    top3 = ranked[:3]
    top3_aucs = {n: scorer_aucs[n]["overall"] for n in top3}

    print(f"Top-3 scorers: {top3}", flush=True)
    for n in top3:
        print(f"  {n}: AUC={top3_aucs[n]:.4f}", flush=True)

    # ------------------------------------------------------------------
    # 2. Z-normalise each top-3 scorer's scores from fixture data
    # ------------------------------------------------------------------
    fixture_is_break = [f["is_break"] for f in fixtures]
    categories = sorted({f["category"] for f in fixtures})

    fixture_z_scores: dict[str, list[float]] = {}
    z_stats: dict[str, tuple[float, float]] = {}
    for name in top3:
        raw = [f["scores"][name] for f in fixtures]
        z, mean, std = _z_normalize(raw)
        fixture_z_scores[name] = z
        z_stats[name] = (mean, std)

    # ------------------------------------------------------------------
    # 3. Simplex grid search
    # ------------------------------------------------------------------
    best_alpha, best_blend_auc = _find_best_alpha(fixture_z_scores, fixture_is_break, top3)
    print(
        f"Best α: {tuple(f'{a:.2f}' for a in best_alpha)}  "
        f"blend AUC={best_blend_auc:.4f}",
        flush=True,
    )

    # ------------------------------------------------------------------
    # 4. Per-category AUC for the blend vs best individual scorer
    # ------------------------------------------------------------------
    n_fixtures = len(fixture_is_break)
    blended_scores = [
        sum(best_alpha[k] * fixture_z_scores[top3[k]][i] for k in range(len(top3)))
        for i in range(n_fixtures)
    ]

    best_individual = ranked[0]

    per_cat_blend: dict[str, float] = {}
    per_cat_best: dict[str, float] = {}
    for cat in categories:
        cat_break_blend = [
            blended_scores[i]
            for i, f in enumerate(fixtures)
            if f["is_break"] and f["category"] == cat
        ]
        cat_ctrl_blend = [
            blended_scores[i]
            for i, f in enumerate(fixtures)
            if not f["is_break"] and f["category"] == cat
        ]
        per_cat_blend[cat] = auc_from_scores(cat_break_blend, cat_ctrl_blend)

        cat_break_best = [
            f["scores"][best_individual]
            for f in fixtures
            if f["is_break"] and f["category"] == cat
        ]
        cat_ctrl_best = [
            f["scores"][best_individual]
            for f in fixtures
            if not f["is_break"] and f["category"] == cat
        ]
        per_cat_best[cat] = auc_from_scores(cat_break_best, cat_ctrl_best)

    # ------------------------------------------------------------------
    # 5. Write report
    # ------------------------------------------------------------------
    out_dir.mkdir(parents=True, exist_ok=True)
    date_str = datetime.date.today().isoformat()
    md_path = out_dir / f"e_blend_{date_str}.md"
    config_path = out_dir / f"blend_config_{date_str}.json"

    beats_winner = best_blend_auc > _PHASE11_WINNER_AUC
    meets_gate = best_blend_auc >= _VICTORY_GATE

    lines: list[str] = []
    lines.append("# Phase 12 S5 — Ensemble Blend Report\n")
    lines.append(f"Date: {date_str}\n")
    lines.append(
        f"> Phase 11 production winner AUC: **{_PHASE11_WINNER_AUC:.4f}**  "
        f"Victory gate: **{_VICTORY_GATE:.2f}**\n"
    )
    lines.append("")

    lines.append("## Top-3 Scorers (by individual AUC)\n")
    lines.append("| rank | scorer | individual_auc |")
    lines.append("|---|---|---|")
    for rank, name in enumerate(top3, 1):
        lines.append(f"| {rank} | {name} | {top3_aucs[name]:.4f} |")
    lines.append("")

    lines.append("## Best Blend\n")
    alpha_dict = dict(zip(top3, (round(a, 4) for a in best_alpha), strict=True))
    lines.append(f"α weights: `{alpha_dict}`\n")
    lines.append(f"Blend AUC: **{best_blend_auc:.4f}**\n")
    lines.append("")

    lines.append("## Per-Category AUC: Blend vs Best Individual\n")
    cat_header = " | ".join(categories)
    lines.append(f"| scorer | {cat_header} |")
    lines.append("|---|" + "---|" * len(categories))
    blend_row = " | ".join(f"{per_cat_blend.get(c, 0.5):.4f}" for c in categories)
    best_row = " | ".join(f"{per_cat_best.get(c, 0.5):.4f}" for c in categories)
    lines.append(f"| blend | {blend_row} |")
    lines.append(f"| {best_individual} | {best_row} |")
    lines.append("")

    lines.append("## Verdict\n")
    lines.append(
        f"- Beats phase-11 winner ({_PHASE11_WINNER_AUC:.4f})? "
        f"**{'YES' if beats_winner else 'NO'}** "
        f"(blend={best_blend_auc:.4f})\n"
    )
    lines.append(
        f"- Meets victory gate (≥ {_VICTORY_GATE:.2f})? "
        f"**{'YES' if meets_gate else 'NO'}**\n"
    )
    if meets_gate:
        lines.append(
            f"> VICTORY: blend AUC {best_blend_auc:.4f} ≥ {_VICTORY_GATE:.2f}. "
            f"Proceed to production promotion.\n"
        )
    elif beats_winner:
        lines.append(
            "> PROGRESS: blend beats the phase-11 winner but does not yet reach the gate. "
            "Consider further scorer development.\n"
        )
    else:
        lines.append(
            "> NO GAIN: blend does not exceed the phase-11 winner AUC. "
            "Investigate scorer quality.\n"
        )

    md_path.write_text("\n".join(lines))
    print(f"Report written to {md_path}", flush=True)

    # ------------------------------------------------------------------
    # 6. Write blend config JSON
    # ------------------------------------------------------------------
    config: dict[str, Any] = {
        "date": date_str,
        "blend_auc": best_blend_auc,
        "beats_phase11_winner": beats_winner,
        "meets_victory_gate": meets_gate,
        "scorers": top3,
        "alphas": list(best_alpha),
        "z_stats": {name: {"mean": z_stats[name][0], "std": z_stats[name][1]} for name in top3},
        "individual_aucs": top3_aucs,
        "per_category_blend_auc": per_cat_blend,
    }
    config_path.write_text(json.dumps(config, indent=2))
    print(f"Blend config written to {config_path}", flush=True)


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Phase 12 S5: ensemble blend of top-3 scorers from bakeoff scores JSON"
    )
    parser.add_argument(
        "--scores",
        required=True,
        help="Path to bakeoff scores JSON (b_scores_<date>.json)",
    )
    parser.add_argument(
        "--out",
        default="docs/research/scoring/signal/phase12",
        help="Output directory for the report and config",
    )
    args = parser.parse_args()

    scores_path = Path(args.scores)
    if not scores_path.exists():
        raise FileNotFoundError(f"Scores file not found: {scores_path}")

    out_dir = Path(args.out)
    print(f"=== Phase 12 S5 blend train  scores={scores_path} ===", flush=True)
    _run_blend_train(scores_path, out_dir)


if __name__ == "__main__":
    main()
