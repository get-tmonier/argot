from __future__ import annotations

import argparse
import datetime
import random
import statistics
import sys
from pathlib import Path
from typing import Any

import argot.research.signal.scorers.jepa_pretrained  # noqa: F401
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

DEFAULT_SIZES = [100, 200, 400, 800, 1200, 1600, 2000]
DEFAULT_SEEDS = [0, 1, 2]


def _run_sweep(
    entry_name: str,
    catalog_dir: Path,
    scorer_name: str,
    sizes: list[int],
    seeds: list[int],
    out_dir: Path,
) -> None:
    entry_dir = catalog_dir / entry_name
    scopes: list[ScopeConfig] = load_scopes(entry_dir)
    specs: list[FixtureSpec] = load_manifest(entry_dir)
    corpus: list[dict[str, Any]] = load_corpus(entry_dir)

    raw_rows: list[tuple[int, int, float]] = []

    for n in sizes:
        for seed in seeds:
            subsampled = random.Random(seed).sample(corpus, min(n, len(corpus)))

            break_scores: list[float] = []
            ctrl_scores: list[float] = []

            for scope in scopes:
                scope_corpus = [
                    r for r in subsampled if r.get("file_path", "").startswith(scope.path_prefix)
                ]
                if not scope_corpus:
                    print(
                        f"  WARNING: n={n} seed={seed} scope={scope.name!r}: "
                        "empty after subsampling — skipping",
                        file=sys.stderr,
                    )
                    continue

                scope_specs = [s for s in specs if s.scope == scope.name]
                fixture_records = [fixture_to_record(entry_dir, spec) for spec in scope_specs]

                scorer = REGISTRY[scorer_name]()
                scorer.fit(scope_corpus)
                scores = scorer.score(fixture_records)

                for idx, spec in enumerate(scope_specs):
                    if spec.is_break:
                        break_scores.append(scores[idx])
                    else:
                        ctrl_scores.append(scores[idx])

            break_mean = statistics.mean(break_scores) if break_scores else 0.0
            ctrl_mean = statistics.mean(ctrl_scores) if ctrl_scores else 0.0
            delta = break_mean - ctrl_mean
            raw_rows.append((n, seed, delta))
            print(f"  n={n:>4} seed={seed} delta={delta:.4f}", flush=True)

    _write_report(entry_name, scorer_name, sizes, seeds, raw_rows, out_dir)


def _write_report(
    entry_name: str,
    scorer_name: str,
    sizes: list[int],
    seeds: list[int],
    raw_rows: list[tuple[int, int, float]],
    out_dir: Path,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    date_str = datetime.date.today().isoformat()
    out_path = out_dir / f"{entry_name}_corpus_sweep_{date_str}.md"

    lines: list[str] = [f"# {entry_name} corpus sweep — {scorer_name}\n"]

    lines.append("## Raw\n")
    lines.append("| n_records | seed | delta | gate |")
    lines.append("|---|---|---|---|")
    for n, seed, delta in raw_rows:
        gate = "✓" if delta >= 0.20 else "✗"
        lines.append(f"| {n} | {seed} | {delta:.4f} | {gate} |")
    lines.append("")

    lines.append("## Summary\n")
    lines.append("| n_records | mean_delta | std_delta | gate |")
    lines.append("|---|---|---|---|")
    for n in sizes:
        deltas = [delta for (rn, _seed, delta) in raw_rows if rn == n]
        if not deltas:
            continue
        mean_delta = statistics.mean(deltas)
        std_delta = statistics.stdev(deltas) if len(deltas) >= 2 else 0.0
        gate = "✓" if mean_delta >= 0.20 else "✗"
        lines.append(f"| {n} | {mean_delta:.4f} | {std_delta:.4f} | {gate} |")
    lines.append("")

    out_path.write_text("\n".join(lines))
    print(f"Report written to {out_path}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description="Corpus-size sweep for signal scorers")
    parser.add_argument("--entry", default="fastapi")
    parser.add_argument("--sizes", default=",".join(str(s) for s in DEFAULT_SIZES))
    parser.add_argument("--seeds", default=",".join(str(s) for s in DEFAULT_SEEDS))
    parser.add_argument("--scorer", default="jepa_pretrained")
    parser.add_argument("--catalog", default=str(CATALOG_DIR))
    parser.add_argument("--out", default="docs/research/scoring/signal")
    args = parser.parse_args()

    sizes = [int(s.strip()) for s in args.sizes.split(",")]
    seeds = [int(s.strip()) for s in args.seeds.split(",")]
    catalog_dir = Path(args.catalog)
    out_dir = Path(args.out)

    print(f"=== corpus sweep: {args.entry} / {args.scorer} ===", flush=True)
    print(f"sizes={sizes} seeds={seeds}", flush=True)

    _run_sweep(args.entry, catalog_dir, args.scorer, sizes, seeds, out_dir)


if __name__ == "__main__":
    main()
