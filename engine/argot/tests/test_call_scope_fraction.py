"""Tests for the call-scope distribution primitive.

Covers:
- Smoke positive-tail: cluster with all top-level calls; hunk with all nested
  calls fires (|tail-z| > 1, contribution > 0).
- Smoke negative-tail: cluster with all nested calls; hunk with all top-level
  calls fires — verifies two-sided behaviour.
- Cluster-size floor: cluster_size < min_cluster_size → 0.0.
- Baseline None: returns 0.0.
- Hunk with 0 calls: abstains (returns 0.0).
- Baseline requires ≥3 files with calls: fewer returns None from fit.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from argot.scoring.scorers.call_scope_fraction import CallScopeFraction, _CallScopeBaseline

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_PY_TOP_LEVEL = "print(1)\nlen([])\nstr(1)\n"
_PY_NESTED = "def foo():\n    print(1)\n    len([])\n    str(1)\n"
_PY_NO_CALLS = "x = 1\ny = x + 2\n"


def _make_py_files(
    tmp_path: Path,
    source: str,
    n: int,
    prefix: str = "file",
) -> list[tuple[Path, str]]:
    """Return n (path, source) tuples, each file written to tmp_path."""
    result: list[tuple[Path, str]] = []
    for i in range(n):
        p = tmp_path / f"{prefix}_{i}.py"
        p.write_text(source, encoding="utf-8")
        result.append((p, source))
    return result


# ---------------------------------------------------------------------------
# Smoke positive-tail
# ---------------------------------------------------------------------------


def test_smoke_positive_tail(tmp_path: Path) -> None:
    """Cluster with all top-level calls; hunk with all nested calls → contribution > 0."""
    primitive = CallScopeFraction()

    cluster = _make_py_files(tmp_path, _PY_TOP_LEVEL, n=10, prefix="top")
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None, "10 files with calls should produce a baseline"

    # Cluster mean should be ≈ 1.0 (all calls top-level), std ≈ 0.0
    assert baseline.mean == pytest.approx(1.0, abs=1e-6)

    # Hunk with all calls inside a function → fraction = 0.0
    contribution = primitive.score(
        _PY_NESTED,
        baseline=baseline,
        cluster_size=10,
    )
    # |tail_z| = |(0.0 - 1.0) / max(0.0, 1e-6)| >> 1 → contribution = clip = 5.0
    assert contribution > 0.0
    assert contribution == pytest.approx(primitive.cluster_bonus_clip)


# ---------------------------------------------------------------------------
# Smoke negative-tail (two-sided verification)
# ---------------------------------------------------------------------------


def test_smoke_negative_tail(tmp_path: Path) -> None:
    """Cluster with all nested calls; hunk with all top-level calls → contribution > 0."""
    primitive = CallScopeFraction()

    cluster = _make_py_files(tmp_path, _PY_NESTED, n=10, prefix="nested")
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None

    # Cluster mean ≈ 0.0 (all calls nested)
    assert baseline.mean == pytest.approx(0.0, abs=1e-6)

    # Hunk with all calls top-level → fraction = 1.0
    contribution = primitive.score(
        _PY_TOP_LEVEL,
        baseline=baseline,
        cluster_size=10,
    )
    # |tail_z| >> 1 → contribution = clip = 5.0
    assert contribution > 0.0
    assert contribution == pytest.approx(primitive.cluster_bonus_clip)


# ---------------------------------------------------------------------------
# Cluster-size floor
# ---------------------------------------------------------------------------


def test_cluster_size_floor(tmp_path: Path) -> None:
    """Cluster_size < min_cluster_size → abstain (0.0), regardless of baseline."""
    primitive = CallScopeFraction()
    assert primitive.min_cluster_size == 10

    cluster = _make_py_files(tmp_path, _PY_TOP_LEVEL, n=10, prefix="top")
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None

    contribution = primitive.score(
        _PY_NESTED,
        baseline=baseline,
        cluster_size=5,  # below min_cluster_size
    )
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Baseline None
# ---------------------------------------------------------------------------


def test_baseline_none_abstains() -> None:
    """When baseline is None, score must return 0.0."""
    primitive = CallScopeFraction()
    # Seed the language so score doesn't short-circuit on missing language
    primitive._language = "python"  # noqa: SLF001
    contribution = primitive.score(
        _PY_NESTED,
        baseline=None,
        cluster_size=20,
    )
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Hunk with 0 calls
# ---------------------------------------------------------------------------


def test_hunk_with_zero_calls(tmp_path: Path) -> None:
    """Hunk with no call expressions → abstain (0.0)."""
    primitive = CallScopeFraction()

    cluster = _make_py_files(tmp_path, _PY_TOP_LEVEL, n=10, prefix="top")
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None

    contribution = primitive.score(
        _PY_NO_CALLS,
        baseline=baseline,
        cluster_size=10,
    )
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# fit_cluster_baseline: fewer than 3 files with calls → None
# ---------------------------------------------------------------------------


def test_fit_returns_none_when_too_few_files_with_calls(tmp_path: Path) -> None:
    """Returns None when fewer than 3 cluster files have ≥1 call expression."""
    primitive = CallScopeFraction()

    # Only 2 files have calls; rest have no calls
    cluster: list[tuple[Path, str]] = []
    for i in range(2):
        p = tmp_path / f"calls_{i}.py"
        p.write_text(_PY_TOP_LEVEL)
        cluster.append((p, _PY_TOP_LEVEL))
    for i in range(8):
        p = tmp_path / f"nocalls_{i}.py"
        p.write_text(_PY_NO_CALLS)
        cluster.append((p, _PY_NO_CALLS))

    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is None


# ---------------------------------------------------------------------------
# Direct score invariants
# ---------------------------------------------------------------------------


def test_score_clipped_at_cluster_bonus_clip(tmp_path: Path) -> None:
    """Contribution never exceeds cluster_bonus_clip even for extreme tail-z."""
    primitive = CallScopeFraction()

    cluster = _make_py_files(tmp_path, _PY_TOP_LEVEL, n=10, prefix="top")
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None

    # |tail_z| is enormous (std ≈ 0.0, so capped by 1e-6 denominator)
    contribution = primitive.score(_PY_NESTED, baseline=baseline, cluster_size=10)
    assert contribution <= primitive.cluster_bonus_clip


def test_score_near_cluster_mean_abstains(tmp_path: Path) -> None:
    """A hunk whose fraction equals the cluster mean contributes 0.0."""
    primitive = CallScopeFraction()

    # Build a mixed cluster: 5 top-level + 5 nested files → mean ≈ 0.5
    top = _make_py_files(tmp_path, _PY_TOP_LEVEL, n=5, prefix="top")
    nested = _make_py_files(tmp_path, _PY_NESTED, n=5, prefix="nested")
    cluster = top + nested
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None

    # A hunk with fraction exactly at the cluster mean → |tail_z| < 1 → 0.0
    # We can't craft exactly mean with source, but we know |tail_z| < 1 gives 0.
    # Use a hunk with fraction = baseline.mean directly via the dataclass.
    hand_baseline = _CallScopeBaseline(mean=0.5, std=0.5)
    # Hunk with fraction 0.5 → tail_z = 0.0 → contribution = 0.0
    # Craft: 2 top-level calls, 2 nested calls → fraction ≈ 0.5
    hunk = "print(1)\nprint(2)\ndef foo():\n    print(3)\n    print(4)\n"
    primitive._language = "python"  # noqa: SLF001
    contribution = primitive.score(hunk, baseline=hand_baseline, cluster_size=10)
    assert contribution == pytest.approx(0.0, abs=0.5)  # within 0.5 of 0 (tail_z < 1)


# ---------------------------------------------------------------------------
# Constant attributes
# ---------------------------------------------------------------------------


def test_primitive_constants() -> None:
    """Verify protocol attribute values are as specified."""
    primitive = CallScopeFraction()
    assert primitive.name == "call_scope_fraction"
    assert primitive.min_cluster_size == 10
    assert primitive.cluster_bonus_clip == pytest.approx(5.0)
