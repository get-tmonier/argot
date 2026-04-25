from __future__ import annotations

import dataclasses
import inspect


def test_bench_alpha_defaults_match_production() -> None:
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer
    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    run_default = fields["call_receiver_alpha"].default

    sig = inspect.signature(build_scorer)
    score_default = sig.parameters["call_receiver_alpha"].default

    scorer_sig = inspect.signature(SequentialImportBpeScorer.__init__)
    scorer_default = scorer_sig.parameters["call_receiver_alpha"].default

    assert run_default == score_default == scorer_default == 2.0, (
        f"Bench defaults drifted from production: run={run_default}, "
        f"score={score_default}, scorer={scorer_default}"
    )


def test_bench_n_cal_default() -> None:
    """RunConfig.n_cal must be 100 (era-10 shipping config)."""
    from argot_bench.run import RunConfig

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    assert fields["n_cal"].default == 100, (
        f"RunConfig.n_cal default is {fields['n_cal'].default}, expected 100. "
        "Bump it back to 100 (era-10 shipping config)."
    )


def test_bench_threshold_percentile_defaults_match_production() -> None:
    """RunConfig, build_scorer, and SequentialImportBpeScorer must all default threshold_percentile=None (max formula, era-10 shipping)."""
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer
    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    run_default = fields["threshold_percentile"].default

    sig = inspect.signature(build_scorer)
    score_default = sig.parameters["threshold_percentile"].default

    scorer_sig = inspect.signature(SequentialImportBpeScorer.__init__)
    scorer_default = scorer_sig.parameters["threshold_percentile"].default

    assert run_default is None and score_default is None and scorer_default is None, (
        f"threshold_percentile defaults drifted: run={run_default}, "
        f"score={score_default}, scorer={scorer_default}. All should be None (max formula, era-10 shipping)."
    )


def test_bench_threshold_iqr_k_defaults_none() -> None:
    """RunConfig.threshold_iqr_k and build_scorer threshold_iqr_k must both default to None."""
    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    assert "threshold_iqr_k" in fields, "RunConfig missing threshold_iqr_k field"
    run_default = fields["threshold_iqr_k"].default
    assert run_default is None, f"RunConfig.threshold_iqr_k default should be None, got {run_default}"

    sig = inspect.signature(build_scorer)
    assert "threshold_iqr_k" in sig.parameters, "build_scorer missing threshold_iqr_k param"
    score_default = sig.parameters["threshold_iqr_k"].default
    assert score_default is None, f"build_scorer threshold_iqr_k default should be None, got {score_default}"


def test_bench_threshold_n_seeds_shipping_default() -> None:
    """RunConfig.threshold_n_seeds and build_scorer threshold_n_seeds must both default to 7 (era-10 shipping config)."""
    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    assert "threshold_n_seeds" in fields, "RunConfig missing threshold_n_seeds field"
    run_default = fields["threshold_n_seeds"].default
    assert run_default == 7, f"RunConfig.threshold_n_seeds default should be 7 (era-10 shipping config), got {run_default}"

    sig = inspect.signature(build_scorer)
    assert "threshold_n_seeds" in sig.parameters, "build_scorer missing threshold_n_seeds param"
    score_default = sig.parameters["threshold_n_seeds"].default
    assert score_default == 7, f"build_scorer threshold_n_seeds default should be 7 (era-10 shipping config), got {score_default}"
