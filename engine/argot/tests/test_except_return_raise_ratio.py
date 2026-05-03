"""Tests for ExceptReturnRaiseRatio.

Mirrors the style of test_shape_primitive_scaffolding.py: pytest,
tmp_path fixtures, no mocks, synthetic corpora.

Coverage:
- Smoke: 10-file Python cluster (9 raise, 1 return inside except) →
  baseline.mean ≈ 0.1, std ≈ 0.3; a hunk that returns inside except
  fires (contribution > 0).
- Cluster-size floor: cluster_size=5 < min_cluster_size=10 → 0.0.
- Baseline=None: explicit pass of baseline=None → 0.0.
- Hunk with no try/except blocks: 0.0 (abstain, math undefined).
- Hunk with try/except but only raises (ratio=0): fires two-sided.
- TypeScript catch_clause: returns are detected via TS grammar.
"""

from __future__ import annotations

import statistics
from pathlib import Path

import pytest

from argot.scoring.scorers.except_return_raise_ratio import ExceptReturnRaiseRatio

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_PY_RAISES_EXCEPT = """\
try:
    risky()
except Exception:
    raise ValueError("bad")
"""

_PY_RETURNS_EXCEPT = """\
try:
    risky()
except Exception:
    return None
"""

_PY_NO_HANDLER = """\
x = 1
y = x + 2
print(y)
"""

_TS_RETURNS_CATCH = """\
try {
  risky();
} catch (e) {
  return null;
}
"""

_TS_THROWS_CATCH = """\
try {
  risky();
} catch (e) {
  throw new Error("bad");
}
"""

_TS_NO_HANDLER = """\
const x = 1;
const y = x + 2;
console.log(y);
"""


def _write_files(tmp_path: Path, contents: list[str], ext: str = ".py") -> list[Path]:
    files = []
    for i, content in enumerate(contents):
        f = tmp_path / f"file_{i}{ext}"
        f.write_text(content)
        files.append(f)
    return files


def _cluster_files(paths: list[Path]) -> list[tuple[Path, str]]:
    return [(p, p.read_text()) for p in paths]


# ---------------------------------------------------------------------------
# Smoke test: 10-file Python cluster (9 raise, 1 return inside except)
# ---------------------------------------------------------------------------


def test_smoke_python_cluster_and_hunk(tmp_path: Path) -> None:
    """9/10 files raise in except, 1/10 returns → mean≈0.1, std≈0.3.

    A hunk that returns inside except → tail-z ≈ +3σ → contribution > 0.
    """
    primitive = ExceptReturnRaiseRatio()
    contents = [_PY_RAISES_EXCEPT] * 9 + [_PY_RETURNS_EXCEPT]
    paths = _write_files(tmp_path, contents)

    baseline = primitive.fit_cluster_baseline(_cluster_files(paths), language="python")
    assert baseline is not None

    # Each file has exactly one return or raise in the except block.
    # 9 files: ratio=0.0  (0 returns, 1 raise)
    # 1 file:  ratio=1.0  (1 return, 0 raises)
    expected_ratios = [0.0] * 9 + [1.0]
    expected_mean = statistics.mean(expected_ratios)
    expected_std = statistics.pstdev(expected_ratios)

    assert baseline.mean == pytest.approx(expected_mean, abs=1e-6)
    assert baseline.std == pytest.approx(expected_std, abs=1e-6)

    # Hunk that returns inside except: ratio=1.0
    contribution = primitive.score(_PY_RETURNS_EXCEPT, baseline=baseline, cluster_size=10)
    tail_z = (1.0 - baseline.mean) / max(baseline.std, 1e-6)
    expected_contribution = min(5.0, max(0.0, abs(tail_z) - 1.0))
    assert contribution == pytest.approx(expected_contribution, abs=1e-6)
    assert contribution > 0.0, "Hunk that fires should score > 0"


# ---------------------------------------------------------------------------
# Cluster-size floor
# ---------------------------------------------------------------------------


def test_cluster_size_floor_returns_zero(tmp_path: Path) -> None:
    """cluster_size < min_cluster_size=10 → primitive returns 0.0."""
    primitive = ExceptReturnRaiseRatio()
    # Use 5 files so fit returns a valid baseline.
    contents = [_PY_RAISES_EXCEPT] * 4 + [_PY_RETURNS_EXCEPT]
    paths = _write_files(tmp_path, contents)
    baseline = primitive.fit_cluster_baseline(_cluster_files(paths), language="python")
    assert baseline is not None  # fit succeeds (≥3 valid files)

    contribution = primitive.score(_PY_RETURNS_EXCEPT, baseline=baseline, cluster_size=5)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Baseline=None
# ---------------------------------------------------------------------------


def test_baseline_none_returns_zero(tmp_path: Path) -> None:
    """Explicit baseline=None → 0.0 regardless of hunk content."""
    primitive = ExceptReturnRaiseRatio()
    contribution = primitive.score(_PY_RETURNS_EXCEPT, baseline=None, cluster_size=20)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Hunk with no try/except blocks
# ---------------------------------------------------------------------------


def test_hunk_with_no_handler_blocks_abstains(tmp_path: Path) -> None:
    """Hunk has no except/catch → ratio undefined → 0.0 (abstain)."""
    primitive = ExceptReturnRaiseRatio()
    contents = [_PY_RAISES_EXCEPT] * 9 + [_PY_RETURNS_EXCEPT]
    paths = _write_files(tmp_path, contents)
    baseline = primitive.fit_cluster_baseline(_cluster_files(paths), language="python")
    assert baseline is not None

    contribution = primitive.score(_PY_NO_HANDLER, baseline=baseline, cluster_size=10)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Insufficient cluster sample → baseline=None
# ---------------------------------------------------------------------------


def test_insufficient_cluster_sample_returns_none(tmp_path: Path) -> None:
    """Fewer than 3 files with defined ratios → fit returns None."""
    primitive = ExceptReturnRaiseRatio()
    # Two files with handlers, rest have none.
    contents = [_PY_RETURNS_EXCEPT, _PY_RAISES_EXCEPT, _PY_NO_HANDLER, _PY_NO_HANDLER]
    paths = _write_files(tmp_path, contents)
    baseline = primitive.fit_cluster_baseline(_cluster_files(paths), language="python")
    assert baseline is None


# ---------------------------------------------------------------------------
# Two-sided: hunk with only raises also fires
# ---------------------------------------------------------------------------


def test_two_sided_raises_only_hunk_fires(tmp_path: Path) -> None:
    """Cluster mostly returns in except; hunk raises → negative tail-z → fires."""
    primitive = ExceptReturnRaiseRatio()
    # 9 return, 1 raise → mean ≈ 0.9
    contents = [_PY_RETURNS_EXCEPT] * 9 + [_PY_RAISES_EXCEPT]
    paths = _write_files(tmp_path, contents)
    baseline = primitive.fit_cluster_baseline(_cluster_files(paths), language="python")
    assert baseline is not None

    # Hunk that only raises inside except: ratio=0.0 (far from mean≈0.9)
    contribution = primitive.score(_PY_RAISES_EXCEPT, baseline=baseline, cluster_size=10)
    assert contribution > 0.0


# ---------------------------------------------------------------------------
# TypeScript: catch_clause
# ---------------------------------------------------------------------------


def test_typescript_catch_clause_returns_detected(tmp_path: Path) -> None:
    """TS files with catch-clause return/throw are correctly measured."""
    primitive = ExceptReturnRaiseRatio()
    # 8 throw, 2 return in catch → mean ≈ 0.2
    contents = [_TS_THROWS_CATCH] * 8 + [_TS_RETURNS_CATCH] * 2
    paths = _write_files(tmp_path, contents, ext=".ts")
    baseline = primitive.fit_cluster_baseline(_cluster_files(paths), language="typescript")
    assert baseline is not None

    expected_ratios = [0.0] * 8 + [1.0] * 2
    expected_mean = statistics.mean(expected_ratios)
    assert baseline.mean == pytest.approx(expected_mean, abs=1e-6)

    # Hunk that returns inside catch: ratio=1.0 → fires
    contribution = primitive.score(_TS_RETURNS_CATCH, baseline=baseline, cluster_size=10)
    assert contribution > 0.0


def test_typescript_no_handler_abstains(tmp_path: Path) -> None:
    """TS hunk with no try/catch → 0.0."""
    primitive = ExceptReturnRaiseRatio()
    contents = [_TS_THROWS_CATCH] * 8 + [_TS_RETURNS_CATCH] * 2
    paths = _write_files(tmp_path, contents, ext=".ts")
    baseline = primitive.fit_cluster_baseline(_cluster_files(paths), language="typescript")
    assert baseline is not None

    contribution = primitive.score(_TS_NO_HANDLER, baseline=baseline, cluster_size=10)
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Protocol-shape attribute checks
# ---------------------------------------------------------------------------


def test_primitive_attributes() -> None:
    """Verify the Protocol-required attributes are present and correct."""
    p = ExceptReturnRaiseRatio()
    assert p.name == "except_return_raise_ratio"
    assert p.min_cluster_size == 10
    assert p.cluster_bonus_clip == 5.0
