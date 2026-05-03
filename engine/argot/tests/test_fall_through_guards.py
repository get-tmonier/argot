"""Tests for FallThroughGuards (fall-through-guard count primitive).

Mirrors test_shape_primitive_scaffolding.py style.

Coverage:
- Smoke positive-tail: cluster with 0-guard functions; hunk with many guards → contribution > 0.
- Smoke neutral: hunk avg matches cluster mean → contribution = 0.
- Cluster-size floor: cluster_size < min_cluster_size → 0.0.
- Baseline=None → 0.0.
- Hunk with no function_definition nodes → 0.0 (abstain).
- Clips at cluster_bonus_clip when tail-z is huge.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from argot.scoring.scorers.fall_through_guards import FallThroughGuards

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_PRIMITIVE = FallThroughGuards()

# A Python function with zero if-guards before its return.
_PY_NO_GUARDS = "def f():\n    return 1\n"

# A Python function with N if-guards before a return.
_PY_MANY_GUARDS = "\n".join(
    ["def f():"] + [f"    if x{i}:\n        pass" for i in range(5)] + ["    return 1"]
)

# A Python snippet with no function definition at all.
_PY_NO_FUNC = "x = 1\n"


def _write_py_no_guards(tmp_path: Path, n: int) -> list[tuple[Path, str]]:
    """Build n cluster files, each with a single function and 0 guards."""
    files: list[tuple[Path, str]] = []
    for i in range(n):
        p = tmp_path / f"f_{i}.py"
        p.write_text(_PY_NO_GUARDS)
        files.append((p, _PY_NO_GUARDS))
    return files


# ---------------------------------------------------------------------------
# Smoke positive-tail
# ---------------------------------------------------------------------------


def test_positive_tail_contribution(tmp_path: Path) -> None:
    """Cluster mean=0 std=0; hunk avg=5 → huge tail-z → clips at cluster_bonus_clip."""
    cluster_files = _write_py_no_guards(tmp_path, 10)
    baseline = _PRIMITIVE.fit_cluster_baseline(cluster_files, "python")
    assert baseline is not None
    assert baseline.mean == pytest.approx(0.0)
    assert baseline.std == pytest.approx(0.0)

    # Hunk: one function with 5 if-guards before a return.
    contribution = _PRIMITIVE.score(_PY_MANY_GUARDS, baseline=baseline, cluster_size=10)
    # std=0 → max(std, 1e-6)=1e-6 → tail_z = 5 / 1e-6 = 5e6 ≫ 0
    # contribution clips at cluster_bonus_clip=5.0
    assert contribution == pytest.approx(_PRIMITIVE.cluster_bonus_clip)


# ---------------------------------------------------------------------------
# Smoke neutral
# ---------------------------------------------------------------------------


def test_neutral_hunk_zero_contribution(tmp_path: Path) -> None:
    """Hunk avg == cluster mean (both 0) → tail-z=0 → contribution=0."""
    cluster_files = _write_py_no_guards(tmp_path, 10)
    baseline = _PRIMITIVE.fit_cluster_baseline(cluster_files, "python")
    assert baseline is not None

    # Hunk also has a function with 0 guards.
    contribution = _PRIMITIVE.score(_PY_NO_GUARDS, baseline=baseline, cluster_size=10)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Cluster-size floor
# ---------------------------------------------------------------------------


def test_cluster_size_floor_abstains(tmp_path: Path) -> None:
    """cluster_size=5 < min_cluster_size=10 → 0.0."""
    cluster_files = _write_py_no_guards(tmp_path, 10)
    baseline = _PRIMITIVE.fit_cluster_baseline(cluster_files, "python")
    assert baseline is not None

    contribution = _PRIMITIVE.score(_PY_MANY_GUARDS, baseline=baseline, cluster_size=5)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Baseline=None
# ---------------------------------------------------------------------------


def test_none_baseline_abstains() -> None:
    """baseline=None → 0.0 regardless of hunk content."""
    contribution = _PRIMITIVE.score(_PY_MANY_GUARDS, baseline=None, cluster_size=20)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Hunk with no function_definition nodes
# ---------------------------------------------------------------------------


def test_no_function_definition_abstains(tmp_path: Path) -> None:
    """Hunk with no function_definition → 0.0 (abstain)."""
    cluster_files = _write_py_no_guards(tmp_path, 10)
    baseline = _PRIMITIVE.fit_cluster_baseline(cluster_files, "python")
    assert baseline is not None

    contribution = _PRIMITIVE.score(_PY_NO_FUNC, baseline=baseline, cluster_size=10)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# fit_cluster_baseline returns None when too few files have functions
# ---------------------------------------------------------------------------


def test_fit_returns_none_below_min_valid_files(tmp_path: Path) -> None:
    """Fewer than 3 files with functions → fit returns None."""
    # Only 2 files with functions.
    cluster_files = _write_py_no_guards(tmp_path, 2)
    # Add files with no functions to bring total up.
    for i in range(5):
        p = tmp_path / f"empty_{i}.py"
        p.write_text(_PY_NO_FUNC)
        cluster_files.append((p, _PY_NO_FUNC))
    baseline = _PRIMITIVE.fit_cluster_baseline(cluster_files, "python")
    assert baseline is None


# ---------------------------------------------------------------------------
# Guard count is zero when function has no return_statement
# ---------------------------------------------------------------------------


def test_function_without_return_contributes_zero_guards(tmp_path: Path) -> None:
    """A function with if-statements but no return_statement counts 0 guards."""
    no_return_func = "def f():\n    if x:\n        pass\n    if y:\n        pass\n"
    cluster_files = _write_py_no_guards(tmp_path, 10)
    baseline = _PRIMITIVE.fit_cluster_baseline(cluster_files, "python")
    assert baseline is not None

    # Per-function guard count for no_return_func is 0 (no return) → avg=0 → neutral.
    contribution = _PRIMITIVE.score(no_return_func, baseline=baseline, cluster_size=10)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Partial guard count (some if-statements before return, some after)
# ---------------------------------------------------------------------------


def test_only_guards_before_first_return_count(tmp_path: Path) -> None:
    """Only if-statements before the FIRST return count."""
    source = (
        "def f():\n"
        "    if a:\n"
        "        pass\n"
        "    if b:\n"
        "        pass\n"
        "    return 1\n"
        "    if c:\n"  # after the return — should NOT count
        "        pass\n"
    )
    cluster_files = _write_py_no_guards(tmp_path, 10)
    baseline = _PRIMITIVE.fit_cluster_baseline(cluster_files, "python")
    assert baseline is not None
    assert baseline.mean == pytest.approx(0.0)

    # hunk avg = 2 guards → tail-z = 2 / 1e-6 → clips at 5.0
    contribution = _PRIMITIVE.score(source, baseline=baseline, cluster_size=10)
    assert contribution == pytest.approx(_PRIMITIVE.cluster_bonus_clip)
