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
    """RunConfig, build_scorer, SequentialImportBpeScorer all default threshold_percentile=None."""
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
        f"score={score_default}, scorer={scorer_default}. All should be None (max formula)."
    )


def test_bench_threshold_iqr_k_defaults_none() -> None:
    """RunConfig.threshold_iqr_k and build_scorer threshold_iqr_k must both default to None."""
    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    assert "threshold_iqr_k" in fields, "RunConfig missing threshold_iqr_k field"
    run_default = fields["threshold_iqr_k"].default
    assert run_default is None, (
        f"RunConfig.threshold_iqr_k default should be None, got {run_default}"
    )

    sig = inspect.signature(build_scorer)
    assert "threshold_iqr_k" in sig.parameters, "build_scorer missing threshold_iqr_k param"
    score_default = sig.parameters["threshold_iqr_k"].default
    assert score_default is None, (
        f"build_scorer threshold_iqr_k default should be None, got {score_default}"
    )


def test_bench_threshold_n_seeds_default() -> None:
    """RunConfig and build_scorer must agree on the multi-seed median N.

    Lowered from 7 to 3 once the threshold proved well-converged at N=3
    across the existing 6 corpora — saved 4 scorer constructions per
    ``build_scorer`` call, which dominates wall time on monorepo-class
    corpora. Users who want N=7 (era-10 shipping config) opt in via
    ``--threshold-n-seeds 7``.
    """
    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    assert "threshold_n_seeds" in fields, "RunConfig missing threshold_n_seeds field"
    run_default = fields["threshold_n_seeds"].default
    assert run_default == 3, (
        f"RunConfig.threshold_n_seeds default should be 3, got {run_default}"
    )

    sig = inspect.signature(build_scorer)
    assert "threshold_n_seeds" in sig.parameters, "build_scorer missing threshold_n_seeds param"
    score_default = sig.parameters["threshold_n_seeds"].default
    assert score_default == 3, (
        f"build_scorer threshold_n_seeds default should be 3, got {score_default}"
    )


def test_bench_cluster_defaults_match_production() -> None:
    """RunConfig, build_scorer, SequentialImportBpeScorer all default to era-11 shipping
    config: n_clusters=8 and cluster_bonus=5.0."""
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    run_n_clusters = fields["call_receiver_n_clusters"].default
    run_cluster_bonus = fields["call_receiver_cluster_bonus"].default

    sig = inspect.signature(build_scorer)
    score_n_clusters = sig.parameters["call_receiver_n_clusters"].default
    score_cluster_bonus = sig.parameters["call_receiver_cluster_bonus"].default

    scorer_sig = inspect.signature(SequentialImportBpeScorer.__init__)
    scorer_n_clusters = scorer_sig.parameters["call_receiver_n_clusters"].default
    scorer_cluster_bonus = scorer_sig.parameters["call_receiver_cluster_bonus"].default

    assert run_n_clusters == score_n_clusters == scorer_n_clusters == 8, (
        f"n_clusters defaults drifted: run={run_n_clusters}, "
        f"score={score_n_clusters}, scorer={scorer_n_clusters} (expected 8)"
    )
    assert run_cluster_bonus == score_cluster_bonus == scorer_cluster_bonus == 5.0, (
        f"cluster_bonus defaults drifted: run={run_cluster_bonus}, "
        f"score={score_cluster_bonus}, scorer={scorer_cluster_bonus} (expected 5.0)"
    )


def test_bench_root_bonus_default() -> None:
    """RunConfig, build_scorer, SequentialImportBpeScorer all default call_receiver_root_bonus=2.0."""
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    assert "call_receiver_root_bonus" in fields, "RunConfig missing call_receiver_root_bonus field"
    run_default = fields["call_receiver_root_bonus"].default

    sig = inspect.signature(build_scorer)
    assert "call_receiver_root_bonus" in sig.parameters, (
        "build_scorer missing call_receiver_root_bonus param"
    )
    score_default = sig.parameters["call_receiver_root_bonus"].default

    scorer_sig = inspect.signature(SequentialImportBpeScorer.__init__)
    assert "call_receiver_root_bonus" in scorer_sig.parameters, (
        "SequentialImportBpeScorer missing call_receiver_root_bonus param"
    )
    scorer_default = scorer_sig.parameters["call_receiver_root_bonus"].default

    assert run_default == score_default == scorer_default == 2.0, (
        f"root_bonus defaults drifted: run={run_default}, score={score_default}, scorer={scorer_default}"
    )
