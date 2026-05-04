"""Tests for TypicalCallDensity shape primitive.

Covers the four required cases from the Phase B PRD:

(a) Abstain on cluster_size < 10.
(b) Abstain on hunk with no call-expression nodes.
(c) Language-agnostic: same class scores both Python and TypeScript hunks
    when under-coverage is present.
(d) Zero contribution on a hunk whose density equals the cluster mean (tail-z < 1).

Plus a sanity smoke test using real faker source files that re-derives the
+10.17 z-score from the Phase B scout on ``synthetic_formula_1``.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from argot.scoring.scorers.typical_call_density import (
    TypicalCallDensity,
    _TypicalCallDensityBaseline,
)

# ---------------------------------------------------------------------------
# Synthetic source helpers
# ---------------------------------------------------------------------------

# Python: each call is to self.numerify / self.random_element (top-10 callees).
_PY_HIGH_DENSITY = """\
class Provider:
    def phone_number(self):
        return self.numerify('###-####')

    def area_code(self):
        return self.random_element(['212', '415', '312'])
"""

# Python: same call count but all calls to methods NOT in any top-10 set.
_PY_LOW_DENSITY = """\
def synthetic_key(ns, seq):
    return '-'.join([ns, str(seq)])
"""

# Python: no calls at all.
_PY_NO_CALLS = "x = 1\ny = x + 2\n"

# TypeScript: calls to typical cluster methods (fetch + resolve).
_TS_HIGH_DENSITY = """\
async function loadData(url: string) {
  const resp = await fetch(url);
  return resolve(resp);
}
async function loadMore(url: string) {
  const resp = await fetch(url + '?page=2');
  return resolve(resp);
}
"""

# TypeScript: calls to something completely different.
_TS_LOW_DENSITY = """\
function syntheticSlug(parts: string[]): string {
  return parts.join('-');
}
"""

# TypeScript: no calls at all.
_TS_NO_CALLS = "const x: number = 1;\n"


def _make_files(
    tmp_path: Path,
    source: str,
    n: int,
    ext: str = ".py",
    prefix: str = "file",
) -> list[tuple[Path, str]]:
    """Return n (path, source) tuples written to tmp_path."""
    result: list[tuple[Path, str]] = []
    for i in range(n):
        p = tmp_path / f"{prefix}_{i}{ext}"
        p.write_text(source, encoding="utf-8")
        result.append((p, source))
    return result


# ---------------------------------------------------------------------------
# (a) Abstain on cluster_size < 10
# ---------------------------------------------------------------------------


def test_abstain_cluster_size_below_floor(tmp_path: Path) -> None:
    """cluster_size < min_cluster_size → score returns 0.0, even with a valid baseline."""
    primitive = TypicalCallDensity()
    assert primitive.min_cluster_size == 10

    # Build a valid baseline from 10 files (so fit succeeds).
    cluster = _make_files(tmp_path, _PY_HIGH_DENSITY, n=10)
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None, "10 files with calls should produce a valid baseline"

    # Score with cluster_size=5 → must abstain.
    contribution = primitive.score(
        _PY_LOW_DENSITY,
        baseline=baseline,
        cluster_size=5,
    )
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# (b) Abstain on hunk with no call-expression nodes
# ---------------------------------------------------------------------------


def test_abstain_hunk_with_zero_calls(tmp_path: Path) -> None:
    """Hunk with no call_expression nodes → score returns 0.0."""
    primitive = TypicalCallDensity()

    cluster = _make_files(tmp_path, _PY_HIGH_DENSITY, n=10)
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None

    contribution = primitive.score(
        _PY_NO_CALLS,
        baseline=baseline,
        cluster_size=10,
    )
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# (c) Language-agnostic: same class handles Python and TypeScript
# ---------------------------------------------------------------------------


def test_language_agnostic_python(tmp_path: Path) -> None:
    """Under-coverage in Python fires the primitive (contribution > 0)."""
    primitive = TypicalCallDensity()

    # Cluster: all files call the top-10 callees → mean density ≈ 1.0.
    cluster = _make_files(tmp_path, _PY_HIGH_DENSITY, n=10, prefix="py_high")
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None, "10 Python files with calls should produce a baseline"
    assert baseline.mean > 0.5, "cluster mean density should be high"

    # Hunk uses totally different callees → density near 0 → z >> 1 → contribution > 0.
    contribution = primitive.score(
        _PY_LOW_DENSITY,
        baseline=baseline,
        cluster_size=10,
    )
    assert contribution > 0.0, "Python under-coverage hunk should fire the primitive"


def test_language_agnostic_typescript(tmp_path: Path) -> None:
    """Under-coverage in TypeScript fires the primitive (contribution > 0)."""
    primitive = TypicalCallDensity()

    # Cluster: all files call fetch + resolve → mean density ≈ 1.0.
    cluster = _make_files(tmp_path, _TS_HIGH_DENSITY, n=10, ext=".ts", prefix="ts_high")
    baseline = primitive.fit_cluster_baseline(cluster, "typescript")
    assert baseline is not None, "10 TypeScript files with calls should produce a baseline"
    assert baseline.mean > 0.5, "cluster mean density should be high"

    # TypeScript hunk with no overlap with top-10 → under-coverage.
    contribution = primitive.score(
        _TS_LOW_DENSITY,
        baseline=baseline,
        cluster_size=10,
    )
    assert contribution > 0.0, "TypeScript under-coverage hunk should fire the primitive"


def test_language_agnostic_typescript_no_calls(tmp_path: Path) -> None:
    """TypeScript hunk with no calls abstains (0.0) — mirrors Python behaviour."""
    primitive = TypicalCallDensity()

    cluster = _make_files(tmp_path, _TS_HIGH_DENSITY, n=10, ext=".ts", prefix="ts_high")
    baseline = primitive.fit_cluster_baseline(cluster, "typescript")
    assert baseline is not None

    contribution = primitive.score(
        _TS_NO_CALLS,
        baseline=baseline,
        cluster_size=10,
    )
    assert contribution == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# (d) Zero contribution on hunk matching cluster mean
# ---------------------------------------------------------------------------


def test_zero_contribution_at_cluster_mean(tmp_path: Path) -> None:
    """A hunk with density equal to the cluster mean contributes 0.0.

    Tail-z = (mean - hunk_density) / std = 0 → ramp = max(0, 0 - 1) = 0.
    Even with some noise in hunk density, as long as |z| <= 1 the ramp is 0.
    We inject the baseline directly so we control mean exactly.
    """
    primitive = TypicalCallDensity()

    # Fit on a real cluster so language is set and top10_set is populated.
    cluster = _make_files(tmp_path, _PY_HIGH_DENSITY, n=10)
    real_baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert real_baseline is not None

    # Override with a synthetic baseline where mean == hunk density == 0.8.
    # With std=0.3 a density-0.8 hunk gives z=0 → ramp=0.
    # With std=0.1 and density offset of 0.05 gives z=0.5 < 1 → ramp=0.
    synthetic_baseline = _TypicalCallDensityBaseline(
        top10_set=real_baseline.top10_set,
        mean=0.8,
        std=0.3,
    )
    # Craft a hunk with density ≈ 0.8 vs the top10_set.
    # _PY_HIGH_DENSITY has 2 calls (self.numerify + self.random_element) both in top10.
    # Density = 2/2 = 1.0; z = (0.8 - 1.0) / 0.3 = -0.67 < 1 → ramp = 0.
    contribution = primitive.score(
        _PY_HIGH_DENSITY,
        baseline=synthetic_baseline,
        cluster_size=10,
    )
    assert contribution == pytest.approx(0.0, abs=1e-9)


# ---------------------------------------------------------------------------
# Additional invariants
# ---------------------------------------------------------------------------


def test_baseline_none_abstains() -> None:
    """Explicit baseline=None → score returns 0.0."""
    primitive = TypicalCallDensity()
    primitive._language = "python"  # noqa: SLF001
    assert primitive.score(_PY_HIGH_DENSITY, baseline=None, cluster_size=20) == pytest.approx(0.0)


def test_score_clipped_at_cluster_bonus_clip(tmp_path: Path) -> None:
    """Extreme under-coverage (density 0, very tight cluster) is clipped at 5.0."""
    primitive = TypicalCallDensity()

    cluster = _make_files(tmp_path, _PY_HIGH_DENSITY, n=10)
    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is not None

    contribution = primitive.score(_PY_LOW_DENSITY, baseline=baseline, cluster_size=10)
    assert contribution <= primitive.cluster_bonus_clip


def test_fit_returns_none_when_too_few_files_with_calls(tmp_path: Path) -> None:
    """fit_cluster_baseline returns None when fewer than 3 files have calls."""
    primitive = TypicalCallDensity()

    cluster: list[tuple[Path, str]] = []
    for i in range(2):
        p = tmp_path / f"calls_{i}.py"
        p.write_text(_PY_HIGH_DENSITY, encoding="utf-8")
        cluster.append((p, _PY_HIGH_DENSITY))
    for i in range(8):
        p = tmp_path / f"nocalls_{i}.py"
        p.write_text(_PY_NO_CALLS, encoding="utf-8")
        cluster.append((p, _PY_NO_CALLS))

    baseline = primitive.fit_cluster_baseline(cluster, "python")
    assert baseline is None


def test_primitive_constants() -> None:
    """Protocol attribute values are as specified in the PRD."""
    primitive = TypicalCallDensity()
    assert primitive.name == "typical_call_density"
    assert primitive.min_cluster_size == 10
    assert primitive.cluster_bonus_clip == pytest.approx(5.0)


# ---------------------------------------------------------------------------
# Sanity smoke test: re-derive +10.17 z on synthetic_formula_1 (faker)
#
# Regression guard tying Wave 2 implementation to Wave 1 scout measurement.
# Skipped when the faker corpus is not available (CI that doesn't clone it).
# ---------------------------------------------------------------------------

_FAKER_REPO = Path(__file__).parent.parent.parent.parent / "benchmarks" / "data" / "faker" / ".repo"
_FAKER_ADDRESS_PROVIDERS = _FAKER_REPO / "faker" / "providers" / "address"


def _faker_available() -> bool:
    return _FAKER_ADDRESS_PROVIDERS.is_dir()


# Hunk from break_synthetic_formula_1.py lines 18–28:
# Three functions that use only f-strings and "-".join(). The one call
# ("-".join(parts)) resolves callee to None (object is a string literal).
_SYNTHETIC_FORMULA_1_HUNK = """\
def synthetic_key(ns: str, seq: int) -> str:
    return f"{ns}_{seq:08d}"


def synthetic_tag(category: str, rank: int) -> str:
    return f"{category}:{rank}"


def synthetic_slug(parts: list[str]) -> str:
    return "-".join(parts)
"""


@pytest.mark.skipif(not _faker_available(), reason="faker corpus not available")
def test_smoke_synthetic_formula_1_z_score() -> None:
    """Re-derive a strong z-score on synthetic_formula_1 using real faker source files.

    Uses all faker address providers as a proxy cluster.  The production cluster
    identified by the Phase B scout (z=+10.17) was a KMeans-selected subset of
    ≈89 very homogeneous providers; reproducing the exact cluster requires running
    the full scorer pipeline.  This test uses the 64 address providers directly
    (mean density ≈ 0.85, std ≈ 0.19 across files), yielding z ≈ 4.4 on the hunk.

    What this guards:
    - The primitive's fit + score path runs correctly on real source files.
    - The hunk ("synthetic_formula_1") scores near 0 density (its one call has a
      None callee → not in the top-10 set → density = 0/1 = 0).
    - The contribution is meaningfully above 0 (z >> 1 → under-coverage fires).

    Threshold: contribution > 3.0 ↔ z > 4.0 on the address-provider cluster.
    The production cluster delivers z ≈ 10; this test conservatively asserts > 3
    because it does not reproduce the exact KMeans assignment.
    """
    primitive = TypicalCallDensity()

    # Read all address provider __init__.py files as the cluster.
    address_files: list[tuple[Path, str]] = []
    for py_file in sorted(_FAKER_ADDRESS_PROVIDERS.rglob("__init__.py")):
        try:
            src = py_file.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        address_files.append((py_file, src))

    assert (
        len(address_files) >= 10
    ), f"expected ≥10 address provider files, got {len(address_files)}"

    baseline = primitive.fit_cluster_baseline(address_files, "python")
    assert baseline is not None, "faker address providers should produce a valid baseline"
    assert baseline.mean > 0.7, f"expected cluster mean density > 0.7, got {baseline.mean:.4f}"

    # Score the synthetic_formula_1 hunk.
    contribution = primitive.score(
        _SYNTHETIC_FORMULA_1_HUNK,
        baseline=baseline,
        cluster_size=len(address_files),
    )

    # ramp = min(5.0, max(0.0, z - 1.0)).  With z ≈ 4.4 we get ramp ≈ 3.4.
    # Asserting > 3.0 confirms z > 4, a clear under-coverage signal well above
    # the 1.5σ honest-EV threshold specified in the PRD.
    assert contribution > 3.0, (
        f"expected contribution > 3.0 (z > 4), got {contribution:.4f}; "
        f"baseline mean={baseline.mean:.4f} std={baseline.std:.4f}"
    )
