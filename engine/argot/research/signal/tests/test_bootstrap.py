from __future__ import annotations

from argot.research.signal.bootstrap import auc_from_scores, paired_bootstrap_ci


def test_auc_empty_lists() -> None:
    """Empty break_scores or ctrl_scores returns 0.5."""
    assert auc_from_scores([], [1.0]) == 0.5
    assert auc_from_scores([1.0], []) == 0.5


def test_auc_perfect_separation() -> None:
    """All break_scores higher than all ctrl_scores → AUC = 1.0."""
    break_scores = [0.8, 0.9, 1.0]
    ctrl_scores = [0.1, 0.2, 0.3]
    assert auc_from_scores(break_scores, ctrl_scores) == 1.0


def test_auc_no_separation() -> None:
    """All break_scores lower than all ctrl_scores → AUC = 0.0."""
    break_scores = [0.1, 0.2, 0.3]
    ctrl_scores = [0.8, 0.9, 1.0]
    assert auc_from_scores(break_scores, ctrl_scores) == 0.0


def test_auc_with_ties() -> None:
    """Some tied scores → AUC is between 0 and 1 (not 0 or 1)."""
    break_scores = [0.5, 0.6, 0.7]
    ctrl_scores = [0.4, 0.5, 0.8]
    auc = auc_from_scores(break_scores, ctrl_scores)
    assert 0.0 < auc < 1.0


def test_paired_bootstrap_ci_structure() -> None:
    """Returns a 3-tuple with ci_low <= delta <= ci_high for a clear-signal case."""
    baseline_break = [0.1, 0.2, 0.3]
    baseline_ctrl = [0.05, 0.15, 0.25]
    variant_break = [0.8, 0.85, 0.9]
    variant_ctrl = [0.1, 0.15, 0.2]

    result = paired_bootstrap_ci(
        baseline_break, baseline_ctrl, variant_break, variant_ctrl, n_resamples=100, seed=42
    )

    assert isinstance(result, tuple)
    assert len(result) == 3
    delta, ci_low, ci_high = result
    assert ci_low <= delta <= ci_high


def test_paired_bootstrap_ci_seed_reproducibility() -> None:
    """Same seed produces same result twice."""
    baseline_break = [0.1, 0.2, 0.3]
    baseline_ctrl = [0.05, 0.15, 0.25]
    variant_break = [0.6, 0.65, 0.7]
    variant_ctrl = [0.1, 0.15, 0.2]

    result1 = paired_bootstrap_ci(
        baseline_break, baseline_ctrl, variant_break, variant_ctrl, n_resamples=50, seed=123
    )

    result2 = paired_bootstrap_ci(
        baseline_break, baseline_ctrl, variant_break, variant_ctrl, n_resamples=50, seed=123
    )

    assert result1 == result2
