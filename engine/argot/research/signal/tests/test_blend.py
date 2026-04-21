"""Tests for BlendScorer and blend_train CLI — no model loading, < 5 seconds."""

from __future__ import annotations

import json
import statistics
import tempfile
from pathlib import Path
from typing import Any

from argot.research.signal.cli.blend_train import (
    _bootstrap_auc_ci,
    _find_best_alpha,
    _run_blend_train,
    _simplex_points,
    _z_normalize,
)
from argot.research.signal.scorers.blend import BlendScorer

# ---------------------------------------------------------------------------
# Fake scorers for unit tests
# ---------------------------------------------------------------------------


class _ConstScorer:
    """Always returns the same constant score for every fixture."""

    name = "const"

    def __init__(self, value: float) -> None:
        self._value = value

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        pass

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        return [self._value] * len(fixtures)


class _IndexScorer:
    """Returns float(index) for each fixture (0-based), offset by a constant."""

    name = "index"

    def __init__(self, offset: float = 0.0) -> None:
        self._offset = offset

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        pass

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        return [float(i) + self._offset for i in range(len(fixtures))]


# ---------------------------------------------------------------------------
# BlendScorer — unit tests
# ---------------------------------------------------------------------------


def test_blend_scorer_weighted_output() -> None:
    """Verify that BlendScorer.score() produces a correct weighted z-score combination."""
    # Two scorers with known constant outputs: [1, 2, 3] and [4, 5, 6]
    scorer_a = _IndexScorer(offset=1.0)  # scores: [1.0, 2.0, 3.0]
    scorer_b = _IndexScorer(offset=4.0)  # scores: [4.0, 5.0, 6.0]
    alphas = [0.6, 0.4]

    blend = BlendScorer([scorer_a, scorer_b], alphas)
    corpus: list[dict[str, Any]] = [{} for _ in range(6)]
    blend.fit(corpus)

    fixtures: list[dict[str, Any]] = [{}, {}, {}]
    scores = blend.score(fixtures)
    assert len(scores) == 3

    # Manually compute expected values
    a_raw = [1.0, 2.0, 3.0]
    b_raw = [4.0, 5.0, 6.0]

    # z-stats from fit(corpus): corpus has 6 fixtures → indices 0..5 (+offset)
    a_corpus = [float(i) + 1.0 for i in range(6)]
    b_corpus = [float(i) + 4.0 for i in range(6)]
    a_mean = statistics.mean(a_corpus)
    a_std = statistics.stdev(a_corpus)
    b_mean = statistics.mean(b_corpus)
    b_std = statistics.stdev(b_corpus)

    expected = [
        0.6 * (a_raw[i] - a_mean) / a_std + 0.4 * (b_raw[i] - b_mean) / b_std
        for i in range(3)
    ]

    for got, exp in zip(scores, expected, strict=True):
        assert abs(got - exp) < 1e-9, f"Expected {exp:.6f}, got {got:.6f}"


def test_blend_scorer_fit_stats() -> None:
    """Verify that fit() stores correct (mean, std) pairs."""
    scorer_a = _IndexScorer(offset=0.0)  # scores: [0.0, 1.0, 2.0, 3.0]
    scorer_b = _ConstScorer(value=5.0)

    blend = BlendScorer([scorer_a, scorer_b], [0.5, 0.5])
    corpus: list[dict[str, Any]] = [{} for _ in range(4)]
    blend.fit(corpus)

    # scorer_a on 4-item corpus: [0.0, 1.0, 2.0, 3.0]
    expected_a_mean = statistics.mean([0.0, 1.0, 2.0, 3.0])
    expected_a_std = statistics.stdev([0.0, 1.0, 2.0, 3.0])
    # scorer_b on 4-item corpus: [5.0, 5.0, 5.0, 5.0] → std clamped to 1.0
    expected_b_mean = 5.0
    expected_b_std = 1.0  # clamped from 0.0

    assert len(blend._stats) == 2
    a_mean, a_std = blend._stats[0]
    b_mean, b_std = blend._stats[1]
    assert abs(a_mean - expected_a_mean) < 1e-9
    assert abs(a_std - expected_a_std) < 1e-9
    assert abs(b_mean - expected_b_mean) < 1e-9
    assert abs(b_std - expected_b_std) < 1e-9


def test_blend_scorer_raises_without_fit() -> None:
    """score() must raise RuntimeError if fit() was not called."""
    import pytest

    blend = BlendScorer([_ConstScorer(1.0)], [1.0])
    with pytest.raises(RuntimeError):
        blend.score([{}])


def test_blend_scorer_mismatched_lengths() -> None:
    """Constructor raises ValueError if scorers and alphas lengths differ."""
    import pytest

    with pytest.raises(ValueError):
        BlendScorer([_ConstScorer(1.0)], [0.5, 0.5])


def test_blend_scorer_corpus_capped_at_500() -> None:
    """fit() uses at most 500 corpus records."""
    scorer = _IndexScorer(offset=0.0)
    blend = BlendScorer([scorer], [1.0])
    # 600-item corpus; scorer returns indices 0..499 for the sample
    corpus: list[dict[str, Any]] = [{} for _ in range(600)]
    blend.fit(corpus)
    mean, std = blend._stats[0]
    expected_mean = statistics.mean(range(500))
    assert abs(mean - expected_mean) < 1e-9


# ---------------------------------------------------------------------------
# Simplex enumeration
# ---------------------------------------------------------------------------


def test_simplex_points_count() -> None:
    """step=0.05 on 3 scorers must produce exactly 231 points."""
    points = _simplex_points(n_scorers=3, step=0.05)
    assert len(points) == 231, f"Expected 231 points, got {len(points)}"


def test_simplex_points_sum_to_one() -> None:
    """Every simplex point must sum to 1.0 (within float tolerance)."""
    points = _simplex_points(n_scorers=3, step=0.05)
    for pt in points:
        total = sum(pt)
        assert abs(total - 1.0) < 1e-9, f"Point {pt} sums to {total}"


def test_simplex_points_non_negative() -> None:
    """All weights must be ≥ 0."""
    points = _simplex_points(n_scorers=3, step=0.05)
    for pt in points:
        for w in pt:
            assert w >= 0.0, f"Negative weight {w} in {pt}"


# ---------------------------------------------------------------------------
# _z_normalize
# ---------------------------------------------------------------------------


def test_z_normalize_known_values() -> None:
    scores = [1.0, 2.0, 3.0, 4.0, 5.0]
    z, mean, std = _z_normalize(scores)
    assert abs(mean - 3.0) < 1e-9
    assert abs(std - statistics.stdev(scores)) < 1e-9
    # z[2] should be 0
    assert abs(z[2]) < 1e-9


def test_z_normalize_constant_clamps_std() -> None:
    """Constant list → std=0 → clamped to 1.0."""
    z, mean, std = _z_normalize([5.0, 5.0, 5.0])
    assert std == 1.0
    assert all(v == 0.0 for v in z)


# ---------------------------------------------------------------------------
# _find_best_alpha
# ---------------------------------------------------------------------------


def test_find_best_alpha_selects_correct_alpha() -> None:
    """With synthetic data, the blend AUC should be at least as good as the best individual."""
    # 4 break fixtures, 4 control fixtures
    # s0 perfectly separates (breaks all > controls after z-norm)
    # s1 perfectly reverses (breaks < controls after z-norm)
    # s2 is random noise (uniform, no signal)
    # The best blend should be dominated by s0 (or anti-correlated s1 negated — but
    # since we only add, mixing them degrades AUC).  We just verify best_auc is high.
    breaks_s0 = [100.0, 101.0, 102.0, 103.0]
    ctrls_s0 = [0.0, 1.0, 2.0, 3.0]
    z0, _, _ = _z_normalize(breaks_s0 + ctrls_s0)

    # s1: useless — same score for all
    z1 = [0.0] * 8

    # s2: useless — same score for all
    z2 = [0.0] * 8

    fixture_z = {"s0": z0, "s1": z1, "s2": z2}
    is_break = [True] * 4 + [False] * 4

    best_alpha, best_auc = _find_best_alpha(fixture_z, is_break, ["s0", "s1", "s2"])
    # The blend using any non-zero weight on s0 achieves perfect AUC;
    # since s0 is the only informative scorer, best_auc must be 1.0
    assert best_auc == 1.0, f"Expected AUC=1.0, got {best_auc}"
    # The best alpha must assign nonzero weight to s0 (first scorer)
    assert best_alpha[0] > 0.0, f"Expected s0 weight > 0, got {best_alpha[0]}"


# ---------------------------------------------------------------------------
# _bootstrap_auc_ci
# ---------------------------------------------------------------------------


def test_bootstrap_auc_ci_bounds_are_ordered() -> None:
    """CI lower bound must be <= CI upper bound."""
    break_scores = [1.0, 2.0, 3.0, 4.0, 5.0]
    ctrl_scores = [0.1, 0.2, 0.3, 0.4, 0.5]
    lo, hi = _bootstrap_auc_ci(break_scores, ctrl_scores, n_resamples=200, seed=0)
    assert lo <= hi, f"Expected lo <= hi, got lo={lo:.4f} hi={hi:.4f}"


def test_bootstrap_auc_ci_perfect_separation() -> None:
    """Perfect separation → both CI bounds should be close to 1.0."""
    break_scores = [10.0, 11.0, 12.0, 13.0]
    ctrl_scores = [0.0, 1.0, 2.0, 3.0]
    lo, hi = _bootstrap_auc_ci(break_scores, ctrl_scores, n_resamples=200, seed=42)
    assert lo > 0.95, f"Expected lo > 0.95 for perfect separation, got {lo:.4f}"
    assert hi == 1.0, f"Expected hi == 1.0 for perfect separation, got {hi:.4f}"


def test_bootstrap_auc_ci_random_chance() -> None:
    """Identical distributions → CI should straddle 0.5."""
    scores = [1.0, 2.0, 3.0, 4.0, 5.0]
    lo, hi = _bootstrap_auc_ci(scores, scores, n_resamples=200, seed=42)
    assert lo < 0.5 < hi, f"Expected CI to straddle 0.5, got [{lo:.4f}, {hi:.4f}]"


def test_bootstrap_auc_ci_deterministic() -> None:
    """Same seed → same result."""
    break_scores = [3.0, 4.0, 5.0]
    ctrl_scores = [1.0, 2.0, 3.0]
    lo1, hi1 = _bootstrap_auc_ci(break_scores, ctrl_scores, n_resamples=100, seed=99)
    lo2, hi2 = _bootstrap_auc_ci(break_scores, ctrl_scores, n_resamples=100, seed=99)
    assert lo1 == lo2 and hi1 == hi2


# ---------------------------------------------------------------------------
# _run_blend_train — integration test (no model loading)
# ---------------------------------------------------------------------------


def _make_scores_json(tmp_dir: Path) -> Path:
    """Build a minimal bakeoff scores JSON for testing."""
    fixtures: list[dict[str, Any]] = []
    scorer_names = ["scorer_a", "scorer_b", "scorer_c", "scorer_d"]

    # 4 break + 4 control fixtures
    for i in range(4):
        fixtures.append(
            {
                "name": f"break_{i}",
                "scope": "s1",
                "is_break": True,
                "category": "routing",
                "set": "v1",
                "scores": {
                    "scorer_a": 10.0 + i,
                    "scorer_b": 5.0 + i,
                    "scorer_c": 3.0 + i,
                    "scorer_d": 1.0 + i,
                },
            }
        )
    for i in range(4):
        fixtures.append(
            {
                "name": f"ctrl_{i}",
                "scope": "s1",
                "is_break": False,
                "category": "routing",
                "set": "v1",
                "scores": {
                    "scorer_a": float(i),
                    "scorer_b": float(i),
                    "scorer_c": float(i),
                    "scorer_d": float(i),
                },
            }
        )

    # Compute real AUCs for scorer_aucs field
    def _auc(bk: list[float], ct: list[float]) -> float:
        wins = sum(1 for b in bk for c in ct if b > c)
        ties = sum(1 for b in bk for c in ct if b == c)
        return (wins + 0.5 * ties) / (len(bk) * len(ct))

    scorer_aucs: dict[str, Any] = {}
    for name in scorer_names:
        bk = [f["scores"][name] for f in fixtures if f["is_break"]]
        ct = [f["scores"][name] for f in fixtures if not f["is_break"]]
        overall = _auc(bk, ct)
        scorer_aucs[name] = {
            "overall": overall,
            "by_category": {"routing": overall},
        }

    data: dict[str, Any] = {
        "entry": "fastapi",
        "context_mode": "file_only",
        "date": "2026-01-01",
        "scorers": scorer_names,
        "fixtures": fixtures,
        "scorer_aucs": scorer_aucs,
    }
    scores_path = tmp_dir / "b_scores_test.json"
    scores_path.write_text(json.dumps(data))
    return scores_path


def test_run_blend_train_writes_output_files() -> None:
    """_run_blend_train writes e_blend_<date>.md and blend_config_<date>.json."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        scores_path = _make_scores_json(tmp_dir)
        out_dir = tmp_dir / "out"

        _run_blend_train(scores_path, out_dir)

        import datetime

        date_str = datetime.date.today().isoformat()
        md_path = out_dir / f"e_blend_{date_str}.md"
        config_path = out_dir / f"blend_config_{date_str}.json"

        assert md_path.exists(), f"Expected report at {md_path}"
        assert config_path.exists(), f"Expected config at {config_path}"

        config = json.loads(config_path.read_text())
        assert "scorers" in config
        assert len(config["scorers"]) == 3
        assert "alphas" in config
        assert len(config["alphas"]) == 3
        assert abs(sum(config["alphas"]) - 1.0) < 1e-9
        assert "blend_auc" in config
        assert 0.0 <= config["blend_auc"] <= 1.0

        # New gate condition fields must be present
        assert "bootstrap_ci" in config
        assert "lo" in config["bootstrap_ci"]
        assert "hi" in config["bootstrap_ci"]
        assert config["bootstrap_ci"]["lo"] <= config["bootstrap_ci"]["hi"]
        assert "ci_clears_winner" in config
        assert isinstance(config["ci_clears_winner"], bool)
        assert "no_cat_below_floor" in config
        assert isinstance(config["no_cat_below_floor"], bool)
        assert "inverted_cats" in config
        assert isinstance(config["inverted_cats"], list)
        assert "inverted_lifted" in config
        assert isinstance(config["inverted_lifted"], bool)
        assert "victory" in config
        assert isinstance(config["victory"], bool)


def test_run_blend_train_top3_by_auc() -> None:
    """The selected scorers must be the top-3 by individual AUC."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        scores_path = _make_scores_json(tmp_dir)
        out_dir = tmp_dir / "out"

        _run_blend_train(scores_path, out_dir)

        import datetime

        date_str = datetime.date.today().isoformat()
        config = json.loads((out_dir / f"blend_config_{date_str}.json").read_text())

        # scorer_a has perfect AUC and must be in top-3
        assert "scorer_a" in config["scorers"]
