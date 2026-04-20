from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import argot.research.signal.scorers.jepa_pretrained  # noqa: F401
import argot.research.signal.scorers.knn_cosine  # noqa: F401
from argot.acceptance.runner import (
    CATALOG_DIR,
    FixtureSpec,
    ScopeConfig,
    fixture_to_record,
    load_corpus,
    load_manifest,
    load_scopes,
)
from argot.research.signal.base import REGISTRY


def _run_entry(
    entry_name: str,
    catalog_dir: Path,
    scorer_names: list[str],
    out_dir: Path,
) -> None:
    entry_dir = catalog_dir / entry_name
    scopes: list[ScopeConfig] = load_scopes(entry_dir)
    specs: list[FixtureSpec] = load_manifest(entry_dir)
    corpus: list[dict[str, Any]] = load_corpus(entry_dir)

    per_fixture: dict[str, dict[str, Any]] = {}

    for scope in scopes:
        scope_corpus = [r for r in corpus if r.get("file_path", "").startswith(scope.path_prefix)]
        scope_specs = [s for s in specs if s.scope == scope.name]

        fixture_records = [fixture_to_record(entry_dir, spec) for spec in scope_specs]

        scorer_scores: dict[str, list[float]] = {}
        for scorer_name in scorer_names:
            scorer = REGISTRY[scorer_name]()
            scorer.fit(scope_corpus)
            scorer_scores[scorer_name] = scorer.score(fixture_records)

        for idx, spec in enumerate(scope_specs):
            entry = per_fixture.setdefault(
                spec.name,
                {"scope": scope.name, "is_break": spec.is_break, "scores": {}},
            )
            for scorer_name in scorer_names:
                entry["scores"][scorer_name] = scorer_scores[scorer_name][idx]

    _write_report(entry_name, per_fixture, scorer_names, out_dir)


def _write_report(
    entry_name: str,
    per_fixture: dict[str, dict[str, Any]],
    scorer_names: list[str],
    out_dir: Path,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [f"# {entry_name}\n"]

    scorer_header = " | ".join(scorer_names)
    lines.append("## Raw Scores\n")
    lines.append(f"| fixture | scope | type | {scorer_header} |")
    lines.append("|---|---|---|" + "---|" * len(scorer_names))
    for fname, data in per_fixture.items():
        ftype = "break" if data["is_break"] else "control"
        score_cols = " | ".join(f"{data['scores'].get(s, 0.0):.4f}" for s in scorer_names)
        lines.append(f"| {fname} | {data['scope']} | {ftype} | {score_cols} |")
    lines.append("")

    total = len(per_fixture)
    rank_scorer_header = " | ".join(f"{s} rank" for s in scorer_names)
    lines.append("## Ranks (1 = most anomalous)\n")
    lines.append(f"| fixture | type | {rank_scorer_header} |")
    lines.append("|---|---|" + "---|" * len(scorer_names))

    scorer_ranks: dict[str, dict[str, int]] = {}
    for scorer_name in scorer_names:
        ordered = sorted(per_fixture.items(), key=lambda kv: kv[1]["scores"].get(scorer_name, 0.0), reverse=True)
        scorer_ranks[scorer_name] = {fname: idx + 1 for idx, (fname, _) in enumerate(ordered)}

    for fname, data in per_fixture.items():
        ftype = "break" if data["is_break"] else "control"
        rank_cols = " | ".join(f"{scorer_ranks[s][fname]}/{total}" for s in scorer_names)
        lines.append(f"| {fname} | {ftype} | {rank_cols} |")
    lines.append("")

    lines.append("## Summary\n")
    lines.append("| scorer | break_mean | ctrl_mean | delta | gate |")
    lines.append("|---|---|---|---|---|")
    for scorer_name in scorer_names:
        break_scores = [d["scores"].get(scorer_name, 0.0) for d in per_fixture.values() if d["is_break"]]
        ctrl_scores = [d["scores"].get(scorer_name, 0.0) for d in per_fixture.values() if not d["is_break"]]
        break_mean = sum(break_scores) / len(break_scores) if break_scores else 0.0
        ctrl_mean = sum(ctrl_scores) / len(ctrl_scores) if ctrl_scores else 0.0
        delta = break_mean - ctrl_mean
        gate = "✓" if delta >= 0.20 else "✗"
        lines.append(f"| {scorer_name} | {break_mean:.4f} | {ctrl_mean:.4f} | {delta:.4f} | {gate} |")
    lines.append("")

    (out_dir / f"{entry_name}.md").write_text("\n".join(lines))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--entry", default=None)
    parser.add_argument("--scorers", default="jepa_pretrained,knn_cosine")
    parser.add_argument("--catalog", default=str(CATALOG_DIR))
    parser.add_argument("--out", default="docs/research/scoring/signal")
    args = parser.parse_args()

    catalog_dir = Path(args.catalog)
    scorer_names = [s.strip() for s in args.scorers.split(",")]
    out_dir = Path(args.out)

    if args.entry:
        entries = [args.entry]
    else:
        entries = [p.name for p in catalog_dir.iterdir() if p.is_dir()]

    for entry_name in entries:
        _run_entry(entry_name, catalog_dir, scorer_names, out_dir)


if __name__ == "__main__":
    main()
