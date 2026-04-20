from __future__ import annotations

import argparse
import datetime
import random
import statistics
import sys
import time
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
from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.signal.base import SignalScorer
from argot.research.signal.scorers.ensemble_jepa import EnsembleJepaScorer
from argot.research.signal.scorers.jepa_custom import JepaCustomScorer
from argot.research.signal.scorers.jepa_filtered import JepaFilteredScorer
from argot.research.signal.scorers.jepa_infonce import JepaInfoNCEScorer
from argot.research.signal.scorers.jepa_pretrained import JepaPretrainedScorer
from argot.train import _texts_for_records

STAGE1_CONFIGS: list[dict[str, Any]] = [
    {"name": "ep20_lr5e5", "epochs": 20, "lr": 5e-5},
    {"name": "ep20_lr1e4", "epochs": 20, "lr": 1e-4},
    {"name": "ep50_lr5e5", "epochs": 50, "lr": 5e-5},
    {"name": "ep50_lr1e4", "epochs": 50, "lr": 1e-4},
    {"name": "ep100_lr5e5", "epochs": 100, "lr": 5e-5},
    {"name": "ep100_lr1e4", "epochs": 100, "lr": 1e-4},
]

# Stage 2: base=ep20_lr1e4, lr_schedule × predictor capacity
# 2 schedules × 4 predictor configs = 8 configs × 3 seeds = 24 runs
STAGE2_CONFIGS: list[dict[str, Any]] = [
    {"name": "flat_d4m512", "lr_schedule": "flat", "depth": 4, "mlp_dim": 512},
    {"name": "flat_d6m512", "lr_schedule": "flat", "depth": 6, "mlp_dim": 512},
    {"name": "flat_d4m1024", "lr_schedule": "flat", "depth": 4, "mlp_dim": 1024},
    {"name": "flat_d6m1024", "lr_schedule": "flat", "depth": 6, "mlp_dim": 1024},
    {"name": "cos_d4m512", "lr_schedule": "cosine", "depth": 4, "mlp_dim": 512},
    {"name": "cos_d6m512", "lr_schedule": "cosine", "depth": 6, "mlp_dim": 512},
    {"name": "cos_d4m1024", "lr_schedule": "cosine", "depth": 4, "mlp_dim": 1024},
    {"name": "cos_d6m1024", "lr_schedule": "cosine", "depth": 6, "mlp_dim": 1024},
]

# Stage 3: corpus filter on top of Stage 2 winner (flat_d4m1024)
# τ ∈ {top-1%, top-5%} = 2 configs × 3 seeds = 6 runs
STAGE3_CONFIGS: list[dict[str, Any]] = [
    {"name": "filtered_tau1", "tau_percentile": 1.0},
    {"name": "filtered_tau5", "tau_percentile": 5.0},
]
# Stage 4: ensemble over flat_d6m1024 (mean=0.221 unensembled, std=0.024)
# N ∈ {3, 5} = 2 configs × 3 base_seeds = 6 runs (each run trains N predictors internally)
STAGE4_CONFIGS: list[dict[str, Any]] = [
    {"name": "ensemble_n3", "n": 3},
    {"name": "ensemble_n5", "n": 5},
]


# Stage 5: top-k surprise aggregation + z-score normalization sweep
# {aggregation ∈ mean, top16, top32, top64, top128} × {zscore ∈ off, on} = 10 configs
# + Goodhart random-k baseline: {rand16, rand32, rand64, rand128} × {zscore ∈ off, on} = 8 configs
# Total: 18 configs × 3 seeds = 54 runs
def _s5(name: str, agg: str, k: int, z: bool, rand: bool) -> dict[str, Any]:
    return {"name": name, "aggregation": agg, "topk_k": k, "zscore": z, "random_k": rand}


STAGE5_CONFIGS: list[dict[str, Any]] = [
    # mean aggregation (baseline comparators)
    _s5("mean_no_z", "mean", 64, False, False),
    _s5("mean_z", "mean", 64, True, False),
    # top-k without z-score
    _s5("top16", "topk", 16, False, False),
    _s5("top32", "topk", 32, False, False),
    _s5("top64", "topk", 64, False, False),
    _s5("top128", "topk", 128, False, False),
    # top-k with z-score
    _s5("top16_z", "topk", 16, True, False),
    _s5("top32_z", "topk", 32, True, False),
    _s5("top64_z", "topk", 64, True, False),
    _s5("top128_z", "topk", 128, True, False),
    # Goodhart random-k baseline (no z-score)
    _s5("rand16", "topk", 16, False, True),
    _s5("rand32", "topk", 32, False, True),
    _s5("rand64", "topk", 64, False, True),
    _s5("rand128", "topk", 128, False, True),
    # Goodhart random-k with z-score
    _s5("rand16_z", "topk", 16, True, True),
    _s5("rand32_z", "topk", 32, True, True),
    _s5("rand64_z", "topk", 64, True, True),
    _s5("rand128_z", "topk", 128, True, True),
]

# Stage 6: InfoNCE loss sweep
# {beta ∈ 0.1, 0.25, 0.5, 1.0} × {tau ∈ 0.05, 0.07, 0.1} × {warmup ∈ 0, 5} = 24 configs × 1 seed
STAGE6_CONFIGS: list[dict[str, Any]] = [
    {
        "name": f"b{str(b).replace('.', '')}_t{str(t).replace('.', '')}_w{w}",
        "beta": b,
        "tau": t,
        "warmup": w,
    }
    for b in [0.1, 0.25, 0.5, 1.0]
    for t in [0.05, 0.07, 0.1]
    for w in [0, 5]
]

STAGE7_CONFIGS: list[dict[str, Any]] = [
    {"name": sampling, "sampling": sampling, "corpus_cap": 2000}
    for sampling in ["linear", "diverse_kmeans", "fps"]
]

_STAGE_CONFIGS: dict[int, list[dict[str, Any]]] = {
    1: STAGE1_CONFIGS,
    2: STAGE2_CONFIGS,
    3: STAGE3_CONFIGS,
    4: STAGE4_CONFIGS,
    5: STAGE5_CONFIGS,
    6: STAGE6_CONFIGS,
    7: STAGE7_CONFIGS,
}

DEFAULT_SEEDS = [0, 1, 2]
DEFAULT_SEEDS_ENSEMBLE = [0]  # n=3 ensemble eliminates variance; single outer seed suffices


def _stage1_factory(cfg: dict[str, Any]) -> SignalScorer:
    return JepaPretrainedScorer(epochs=int(cfg["epochs"]), lr=float(cfg["lr"]))


def _stage2_factory(cfg: dict[str, Any]) -> SignalScorer:
    return JepaCustomScorer(
        epochs=20,
        lr=1e-4,
        lr_schedule=str(cfg["lr_schedule"]),  # type: ignore[arg-type]
        predictor_overrides={"depth": int(cfg["depth"]), "mlp_dim": int(cfg["mlp_dim"])},
    )


def _stage3_factory(cfg: dict[str, Any]) -> SignalScorer:
    return JepaFilteredScorer(tau_percentile=float(cfg["tau_percentile"]))


def _stage4_factory(cfg: dict[str, Any]) -> SignalScorer:
    return EnsembleJepaScorer(n=int(cfg["n"]))


def _stage5_factory(cfg: dict[str, Any]) -> SignalScorer:
    agg = "random_topk" if cfg["random_k"] else str(cfg["aggregation"])
    return EnsembleJepaScorer(
        n=3,
        aggregation=agg,  # type: ignore[arg-type]
        topk_k=int(cfg["topk_k"]),
        zscore_vs_corpus=bool(cfg["zscore"]),
    )


class _EnsembleInfoNCE:
    """Ensemble of JepaInfoNCEScorer for Stage 6/7 sweep."""

    def __init__(
        self,
        *,
        n: int = 3,
        beta: float,
        tau: float,
        warmup_epochs: int,
        sampling: str = "linear",
        corpus_cap: int = 2000,
    ) -> None:
        self._n = n
        self._beta = beta
        self._tau = tau
        self._warmup_epochs = warmup_epochs
        self._sampling = sampling
        self._corpus_cap = corpus_cap
        self._members: list[JepaInfoNCEScorer] = []

    def fit(
        self,
        corpus: list[dict[str, Any]],
        *,
        preencoded: tuple[torch.Tensor, torch.Tensor] | None = None,
    ) -> None:
        self._members = []
        for i in range(self._n):
            m = JepaInfoNCEScorer(
                beta=self._beta,
                tau=self._tau,
                warmup_epochs=self._warmup_epochs,
                random_seed=i,
                sampling=self._sampling,  # type: ignore[arg-type]
                corpus_cap=self._corpus_cap,
            )
            m.fit(corpus, preencoded=preencoded)
            self._members.append(m)

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        all_scores = [m.score(fixtures) for m in self._members]
        return [sum(run[i] for run in all_scores) / self._n for i in range(len(fixtures))]


def _stage6_factory(cfg: dict[str, Any]) -> SignalScorer:
    return _EnsembleInfoNCE(  # type: ignore[return-value]
        n=3,
        beta=float(cfg["beta"]),
        tau=float(cfg["tau"]),
        warmup_epochs=int(cfg["warmup"]),
    )


def _stage7_factory(cfg: dict[str, Any]) -> SignalScorer:
    return _EnsembleInfoNCE(  # type: ignore[return-value]
        n=3,
        beta=0.1,
        tau=0.1,
        warmup_epochs=0,
        sampling=str(cfg["sampling"]),
        corpus_cap=int(cfg["corpus_cap"]),
    )


_STAGE_FACTORIES: dict[int, Callable[[dict[str, Any]], SignalScorer]] = {
    1: _stage1_factory,
    2: _stage2_factory,
    3: _stage3_factory,
    4: _stage4_factory,
    5: _stage5_factory,
    6: _stage6_factory,
    7: _stage7_factory,
}


def _run_sweep(
    stage: int,
    entry_name: str,
    catalog_dir: Path,
    configs: list[dict[str, Any]],
    factory: Callable[[dict[str, Any]], SignalScorer],
    seeds: list[int],
    out_dir: Path,
    corpus_cap: int = 2000,
) -> None:
    entry_dir = catalog_dir / entry_name
    scopes: list[ScopeConfig] = load_scopes(entry_dir)
    specs: list[FixtureSpec] = load_manifest(entry_dir)
    corpus: list[dict[str, Any]] = load_corpus(entry_dir)
    corpus = corpus[:corpus_cap]

    # Pre-encode corpus per scope once (reused across all configs and seeds)
    print("Pre-encoding corpus per scope (runs once)...", flush=True)
    scope_sorted_corpus: dict[str, list[dict[str, Any]]] = {}
    scope_preencoded: dict[str, tuple[torch.Tensor, torch.Tensor]] = {}
    for scope in scopes:
        sc = sorted(
            [r for r in corpus if r.get("file_path", "").startswith(scope.path_prefix)],
            key=lambda r: int(r["author_date_iso"]),
        )
        if not sc:
            continue
        scope_sorted_corpus[scope.name] = sc
        device = select_device()
        pretrained = PretrainedEncoder(device=device)
        ctx_texts, hunk_texts = _texts_for_records(sc)
        print(f"  scope={scope.name!r}  {len(sc)}×2 texts ...", flush=True)
        t0 = time.perf_counter()
        with torch.no_grad():
            ctx_x = pretrained.encode_texts(ctx_texts).cpu()
            hunk_x = pretrained.encode_texts(hunk_texts).cpu()
        del pretrained
        print(f"  done in {time.perf_counter() - t0:.1f}s", flush=True)
        scope_preencoded[scope.name] = (ctx_x, hunk_x)

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
                scope_corpus = scope_sorted_corpus.get(scope.name)
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
                # Stage 3 filter scorers need break fixtures before fit()
                if hasattr(scorer, "prime_breaks"):
                    break_records = [
                        fixture_to_record(entry_dir, s) for s in scope_specs if s.is_break
                    ]
                    scorer.prime_breaks(break_records)
                scorer.fit(scope_corpus, preencoded=scope_preencoded.get(scope.name))  # type: ignore[call-arg]
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
    parser.add_argument("--stage", type=int, choices=[1, 2, 3, 4, 5, 6, 7], default=1)
    parser.add_argument("--entry", default="fastapi")
    # Parse stage early to set the correct default seeds before building the parser default
    _pre = argparse.ArgumentParser(add_help=False)
    _pre.add_argument("--stage", type=int, default=1)
    _pre_args, _ = _pre.parse_known_args()
    _default_seeds = DEFAULT_SEEDS_ENSEMBLE if _pre_args.stage >= 5 else DEFAULT_SEEDS
    parser.add_argument("--seeds", default=",".join(str(s) for s in _default_seeds))
    parser.add_argument("--catalog", default=str(CATALOG_DIR))
    parser.add_argument("--out", default="docs/research/scoring/signal")
    parser.add_argument("--start-from", default=None, help="Skip configs before this name")
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

    if args.start_from is not None:
        names = [str(c["name"]) for c in configs]
        if args.start_from not in names:
            raise ValueError(f"--start-from {args.start_from!r} not found in stage {stage} configs")
        configs = configs[names.index(args.start_from) :]

    print(f"=== JEPA sweep: stage={stage} entry={args.entry} seeds={seeds} ===", flush=True)

    max_cap = max((int(c.get("corpus_cap", 2000)) for c in configs), default=2000)
    _run_sweep(stage, args.entry, catalog_dir, configs, factory, seeds, out_dir, corpus_cap=max_cap)


if __name__ == "__main__":
    main()
