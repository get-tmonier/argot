from __future__ import annotations

import argparse
import datetime
import random
import statistics
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any

import numpy as np
import torch

from argot.acceptance.runner import (
    CATALOG_DIR,
    FixtureSpec,
    ScopeConfig,
    fixture_to_record,
    load_corpus,
    load_manifest,
    load_scopes,
)
from argot.research.signal.base import SignalScorer
from argot.research.signal.scorers.jepa_pretrained import JepaPretrainedScorer

STAGE1_CONFIGS: list[dict[str, Any]] = [
    {"name": "ep20_lr5e5", "epochs": 20, "lr": 5e-5},
    {"name": "ep20_lr1e4", "epochs": 20, "lr": 1e-4},
    {"name": "ep50_lr5e5", "epochs": 50, "lr": 5e-5},
    {"name": "ep50_lr1e4", "epochs": 50, "lr": 1e-4},
    {"name": "ep100_lr5e5", "epochs": 100, "lr": 5e-5},
    {"name": "ep100_lr1e4", "epochs": 100, "lr": 1e-4},
]

STAGE2_CONFIGS: list[dict[str, Any]] = []
STAGE3_CONFIGS: list[dict[str, Any]] = []
STAGE4_CONFIGS: list[dict[str, Any]] = []

_STAGE_CONFIGS: dict[int, list[dict[str, Any]]] = {
    1: STAGE1_CONFIGS,
    2: STAGE2_CONFIGS,
    3: STAGE3_CONFIGS,
    4: STAGE4_CONFIGS,
}

DEFAULT_SEEDS = [0, 1, 2]


def _stage1_factory(cfg: dict[str, Any]) -> SignalScorer:
    return JepaPretrainedScorer(epochs=int(cfg["epochs"]), lr=float(cfg["lr"]))


_STAGE_FACTORIES: dict[int, Callable[[dict[str, Any]], SignalScorer]] = {
    1: _stage1_factory,
}


def _run_sweep(
    stage: int,
    entry_name: str,
    catalog_dir: Path,
    configs: list[dict[str, Any]],
    factory: Callable[[dict[str, Any]], SignalScorer],
    seeds: list[int],
    out_dir: Path,
) -> None:
    entry_dir = catalog_dir / entry_name
    scopes: list[ScopeConfig] = load_scopes(entry_dir)
    specs: list[FixtureSpec] = load_manifest(entry_dir)
    corpus: list[dict[str, Any]] = load_corpus(entry_dir)
    corpus = corpus[:2000]

    # Raw rows: (config_name, seed, delta)
    raw_rows: list[tuple[str, int, float]] = []

    for cfg in configs:
        config_name: str = str(cfg["name"])
        for seed in seeds:
            # Seed all RNGs at the start of each run
            random.seed(seed)
            np.random.seed(seed)
            torch.manual_seed(seed)

            break_scores: list[float] = []
            ctrl_scores: list[float] = []

            for scope in scopes:
                scope_corpus = [
                    r for r in corpus if r.get("file_path", "").startswith(scope.path_prefix)
                ]
                if not scope_corpus:
                    print(
                        f"  WARNING: config={config_name!r} seed={seed} scope={scope.name!r}: "
                        "empty corpus — skipping",
                        file=sys.stderr,
                    )
                    continue

                scope_specs = [s for s in specs if s.scope == scope.name]
                fixture_records = [fixture_to_record(entry_dir, spec) for spec in scope_specs]

                scorer = factory(cfg)
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
            raw_rows.append((config_name, seed, delta))
            print(f"  config={config_name!r} seed={seed} delta={delta:.4f}", flush=True)

    _write_report(stage, entry_name, configs, seeds, raw_rows, out_dir)


def _write_report(
    stage: int,
    entry_name: str,
    configs: list[dict[str, Any]],
    seeds: list[int],
    raw_rows: list[tuple[str, int, float]],
    out_dir: Path,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    date_str = datetime.date.today().isoformat()
    out_path = out_dir / f"sweep_{entry_name}_stage{stage}_{date_str}.md"

    config_names = [str(cfg["name"]) for cfg in configs]

    lines: list[str] = [f"# {entry_name} stage {stage} sweep\n"]

    lines.append("## Raw\n")
    lines.append("| config | seed | delta | gate |")
    lines.append("|---|---|---|---|")
    for config_name, seed, delta in raw_rows:
        gate = "✓" if delta >= 0.20 else "✗"
        lines.append(f"| {config_name} | {seed} | {delta:.4f} | {gate} |")
    lines.append("")

    lines.append("## Summary\n")
    lines.append("| config | mean_delta | std_delta | gate |")
    lines.append("|---|---|---|---|")
    for config_name in config_names:
        deltas = [delta for (cn, _seed, delta) in raw_rows if cn == config_name]
        if not deltas:
            continue
        mean_delta = statistics.mean(deltas)
        std_delta = statistics.stdev(deltas) if len(deltas) >= 2 else 0.0
        gate = "✓" if mean_delta >= 0.20 else "✗"
        lines.append(f"| {config_name} | {mean_delta:.4f} | {std_delta:.4f} | {gate} |")
    lines.append("")

    out_path.write_text("\n".join(lines))
    print(f"Report written to {out_path}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description="JEPA fine-tuning hyperparameter sweep")
    parser.add_argument("--stage", type=int, choices=[1, 2, 3, 4], default=1)
    parser.add_argument("--entry", default="fastapi")
    parser.add_argument("--seeds", default=",".join(str(s) for s in DEFAULT_SEEDS))
    parser.add_argument("--catalog", default=str(CATALOG_DIR))
    parser.add_argument("--out", default="docs/research/scoring/signal")
    args = parser.parse_args()

    stage: int = args.stage
    seeds = [int(s.strip()) for s in args.seeds.split(",")]
    catalog_dir = Path(args.catalog)
    out_dir = Path(args.out)

    if stage not in _STAGE_FACTORIES:
        raise NotImplementedError(f"Stage {stage} not yet implemented")

    configs = _STAGE_CONFIGS[stage]
    factory = _STAGE_FACTORIES[stage]

    if not configs:
        raise NotImplementedError(f"Stage {stage} not yet implemented")

    print(f"=== JEPA sweep: stage={stage} entry={args.entry} seeds={seeds} ===", flush=True)

    _run_sweep(stage, args.entry, catalog_dir, configs, factory, seeds, out_dir)


if __name__ == "__main__":
    main()
