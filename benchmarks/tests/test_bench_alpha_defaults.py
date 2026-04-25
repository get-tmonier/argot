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
    """RunConfig.n_cal must be 300 (era-10 calibration hardening)."""
    from argot_bench.run import RunConfig

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    assert fields["n_cal"].default == 300, (
        f"RunConfig.n_cal default is {fields['n_cal'].default}, expected 300. "
        "Bump it back to 300 (era-10 calibration hardening)."
    )


def test_bench_threshold_percentile_defaults_match_production() -> None:
    """RunConfig, build_scorer, and SequentialImportBpeScorer must all default threshold_percentile=95.0."""
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer
    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    run_default = fields["threshold_percentile"].default

    sig = inspect.signature(build_scorer)
    score_default = sig.parameters["threshold_percentile"].default

    scorer_sig = inspect.signature(SequentialImportBpeScorer.__init__)
    scorer_default = scorer_sig.parameters["threshold_percentile"].default

    assert run_default == score_default == scorer_default == 95.0, (
        f"threshold_percentile defaults drifted: run={run_default}, "
        f"score={score_default}, scorer={scorer_default}. Update all to 95.0."
    )


def test_bench_threshold_iqr_k_defaults_none() -> None:
    """RunConfig.threshold_iqr_k and build_scorer threshold_iqr_k must both default to None."""
    import dataclasses
    import inspect

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
