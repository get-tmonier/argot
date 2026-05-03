"""Tests for NamespaceJsd (receiver-namespace coverage divergence).

Mirrors the test style in test_shape_primitive_scaffolding.py.

Coverage:
- smoke_divergent: cluster dominated by "Math", hunk uses only "fetch"/"axios"
  (all OOV) → JS distance ≈ 1.0 → contribution ≈ cluster_bonus_clip.
- smoke_identical: hunk namespace mix matches cluster exactly → JSD = 0 → contribution = 0.
- cluster_size_floor: cluster_size < min_cluster_size → 0.0.
- baseline_none: score() with baseline=None → 0.0.
- hunk_zero_callees: hunk with no call expressions → 0.0 (abstain).
- single_namespace_cluster: fit returns None when alphabet < 2.
- sparse_cluster: fit returns None when fewer than 3 files have callees.
- fires_through_dispatch: end-to-end integration via CallReceiverScorer.
"""

from __future__ import annotations

from pathlib import Path
from typing import Literal

import pytest

from argot.scoring.scorers.call_receiver import CallReceiverScorer
from argot.scoring.scorers.namespace_jsd import NamespaceJsd, _NamespaceBaseline

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_LANG: Literal["typescript"] = "typescript"


def _make_ts_file(tmp_path: Path, name: str, content: str) -> tuple[Path, str]:
    p = tmp_path / name
    p.write_text(content)
    return p, content


def _make_corpus(tmp_path: Path) -> list[tuple[Path, str]]:
    """10-file cluster: 9 files call only Math.X, 1 file calls Date.now.

    Cluster namespace distribution: Math = 9/10 = 0.9, Date = 1/10 = 0.1.
    """
    files = []
    for i in range(9):
        files.append(_make_ts_file(tmp_path, f"math_{i}.ts", "Math.floor(x);"))
    files.append(_make_ts_file(tmp_path, "date_0.ts", "Date.now();"))
    return files


# ---------------------------------------------------------------------------
# fit_cluster_baseline guards
# ---------------------------------------------------------------------------


def test_single_namespace_returns_none(tmp_path: Path) -> None:
    """Alphabet with < 2 namespaces: fit returns None (JSD ill-defined)."""
    prim = NamespaceJsd()
    # All 10 files use only Math — alphabet = {"Math"}, size 1.
    cluster_files = [
        _make_ts_file(tmp_path, f"m{i}.ts", "Math.floor(x); Math.random();") for i in range(10)
    ]
    result = prim.fit_cluster_baseline(cluster_files, _LANG)
    assert result is None


def test_sparse_cluster_returns_none(tmp_path: Path) -> None:
    """Fewer than 3 files with callees: fit returns None."""
    prim = NamespaceJsd()
    # 2 files with callees (Math + Date), rest are empty/comment-only.
    cluster_files = [
        _make_ts_file(tmp_path, "math_0.ts", "Math.floor(x);"),
        _make_ts_file(tmp_path, "date_0.ts", "Date.now();"),
        _make_ts_file(tmp_path, "empty.ts", "// no calls"),
    ]
    result = prim.fit_cluster_baseline(cluster_files, _LANG)
    assert result is None


def test_fit_returns_valid_baseline(tmp_path: Path) -> None:
    """Happy path: fit returns a baseline with the correct alphabet."""
    prim = NamespaceJsd()
    cluster_files = _make_corpus(tmp_path)
    baseline = prim.fit_cluster_baseline(cluster_files, _LANG)
    assert baseline is not None
    assert "Math" in baseline.alphabet
    assert "Date" in baseline.alphabet
    assert len(baseline.alphabet) == 2
    assert abs(sum(baseline.distribution.values()) - 1.0) < 1e-9
    assert baseline.distribution["Math"] == pytest.approx(0.9)
    assert baseline.distribution["Date"] == pytest.approx(0.1)


# ---------------------------------------------------------------------------
# score() guards
# ---------------------------------------------------------------------------


def test_score_baseline_none_returns_zero(tmp_path: Path) -> None:
    """score() with baseline=None always returns 0.0."""
    prim = NamespaceJsd()
    result = prim.score("fetch(url);", baseline=None, cluster_size=20)
    assert result == 0.0


def test_score_below_min_cluster_size_returns_zero(tmp_path: Path) -> None:
    """score() with cluster_size < min_cluster_size returns 0.0."""
    prim = NamespaceJsd()
    baseline = _NamespaceBaseline(
        language=_LANG,
        alphabet=frozenset({"Math", "Date"}),
        distribution={"Math": 0.9, "Date": 0.1},
    )
    # min_cluster_size = 10; pass cluster_size = 5 → abstain.
    result = prim.score("fetch(url);", baseline=baseline, cluster_size=5)
    assert result == 0.0


def test_score_hunk_zero_callees_returns_zero(tmp_path: Path) -> None:
    """Hunk with no call expressions → abstain (JSD undefined)."""
    prim = NamespaceJsd()
    baseline = _NamespaceBaseline(
        language=_LANG,
        alphabet=frozenset({"Math", "Date"}),
        distribution={"Math": 0.9, "Date": 0.1},
    )
    # Pure comment / variable declaration — no call expressions.
    result = prim.score("const x = 1; // no calls", baseline=baseline, cluster_size=20)
    assert result == 0.0


# ---------------------------------------------------------------------------
# JSD correctness
# ---------------------------------------------------------------------------


def test_smoke_divergent_distribution(tmp_path: Path) -> None:
    """Hunk uses only OOV namespaces → JSD = 1.0 → contribution = cluster_bonus_clip.

    Cluster alphabet = {Math, Date}.
    Hunk calls only fetch() and axios.get() — both OOV.
    Projected distributions:
      cluster = [Math: 0.9, Date: 0.1, OOV: 0.0]
      hunk    = [Math: 0.0, Date: 0.0, OOV: 1.0]
    JSD = 1.0 bit (base-2), JS distance = 1.0, contribution = 5.0.
    """
    prim = NamespaceJsd()
    cluster_files = _make_corpus(tmp_path)
    baseline = prim.fit_cluster_baseline(cluster_files, _LANG)
    assert baseline is not None

    hunk = "fetch(url); axios.get(path);"
    contribution = prim.score(hunk, baseline=baseline, cluster_size=10)

    # JS distance between disjoint distributions = 1.0 → full bonus.
    assert contribution == pytest.approx(prim.cluster_bonus_clip, abs=1e-6)


def test_smoke_identical_distribution(tmp_path: Path) -> None:
    """Hunk's namespace mix matches cluster pooled distribution → JSD = 0 → contribution = 0.

    Cluster: Math=9, Date=1 → {Math: 0.9, Date: 0.1}.
    Hunk: 9 Math calls + 1 Date call → same distribution → JSD = 0.
    """
    prim = NamespaceJsd()
    cluster_files = _make_corpus(tmp_path)
    baseline = prim.fit_cluster_baseline(cluster_files, _LANG)
    assert baseline is not None

    # 9 Math calls + 1 Date call mirrors the cluster distribution exactly.
    hunk = (
        "Math.floor(a); Math.floor(b); Math.floor(c); "
        "Math.floor(d); Math.floor(e); Math.floor(f); "
        "Math.floor(g); Math.floor(h); Math.floor(i); "
        "Date.now();"
    )
    contribution = prim.score(hunk, baseline=baseline, cluster_size=10)
    assert contribution == pytest.approx(0.0, abs=1e-9)


def test_partial_oov_intermediate_contribution(tmp_path: Path) -> None:
    """Hunk with partial OOV yields JSD strictly between 0 and 1."""
    prim = NamespaceJsd()
    cluster_files = _make_corpus(tmp_path)
    baseline = prim.fit_cluster_baseline(cluster_files, _LANG)
    assert baseline is not None

    # Mix: some Math calls (in alphabet) + some fetch calls (OOV).
    hunk = "Math.floor(x); Math.floor(y); fetch(url);"
    contribution = prim.score(hunk, baseline=baseline, cluster_size=10)
    assert 0.0 < contribution < prim.cluster_bonus_clip


# ---------------------------------------------------------------------------
# End-to-end integration via CallReceiverScorer
# ---------------------------------------------------------------------------


def _make_file_corpus(tmp_path: Path) -> list[Path]:
    """20-file corpus: two natural clusters, each with ≥ 2 distinct namespaces.

    Math cluster (files 0-9): "Math.X" + "Date.Y" calls → alphabet {Math, Date}.
    Fetch cluster (files 10-19): "fetch" + "Promise" + "console" calls.
    Each cluster has 10 files with callees (≥ 3 threshold met).
    """
    files: list[Path] = []
    for i in range(10):
        p = tmp_path / f"math_{i}.ts"
        p.write_text("Math.floor(x); Math.random(); Date.now();")
        files.append(p)
    for i in range(10):
        p = tmp_path / f"fetch_{i}.ts"
        p.write_text("fetch(url); Promise.resolve(x); console.log('done');")
        files.append(p)
    return files


def test_fires_through_call_receiver_dispatch(tmp_path: Path) -> None:
    """NamespaceJsd plugs into CallReceiverScorer and increments fire counter."""
    files = _make_file_corpus(tmp_path)
    prim = NamespaceJsd()

    scorer_no_prim = CallReceiverScorer(files, language=_LANG, n_clusters=2)
    scorer_with = CallReceiverScorer(files, language=_LANG, n_clusters=2, shape_primitives=[prim])

    # Primitive should have been fitted on 2 clusters.
    assert prim.name in scorer_with.primitive_baselines

    # Score a hunk that is highly divergent from its cluster.
    # Files 0-9 are Math files and cluster around Math.
    # Hunk uses only console.log (different namespace) — should fire.
    hunk_divergent = "axios.get(path); superagent.post(url);"
    file_path = files[0]  # Math cluster file

    score_no_prim = scorer_no_prim.weighted_contribution_for_file(
        hunk_divergent, file_path, alpha=2.0, root_bonus=2.0, cluster_bonus=5.0, cap=20.0
    )
    score_with = scorer_with.weighted_contribution_for_file(
        hunk_divergent, file_path, alpha=2.0, root_bonus=2.0, cluster_bonus=5.0, cap=20.0
    )
    # Primitive must have added a positive contribution.
    assert score_with >= score_no_prim
    assert scorer_with.primitive_fire_count[prim.name] >= 1


def test_cluster_size_floor_via_dispatch(tmp_path: Path) -> None:
    """Primitive with min_cluster_size=50 abstains on a 10-file cluster."""
    files = _make_file_corpus(tmp_path)
    prim = NamespaceJsd(min_cluster_size=50)

    scorer_no_prim = CallReceiverScorer(files, language=_LANG, n_clusters=2)
    scorer_with = CallReceiverScorer(files, language=_LANG, n_clusters=2, shape_primitives=[prim])

    hunk = "Math.floor(x);"
    file_path = files[0]

    score_no = scorer_no_prim.weighted_contribution_for_file(
        hunk, file_path, alpha=2.0, root_bonus=2.0, cluster_bonus=5.0, cap=20.0
    )
    score_with = scorer_with.weighted_contribution_for_file(
        hunk, file_path, alpha=2.0, root_bonus=2.0, cluster_bonus=5.0, cap=20.0
    )
    assert score_with == pytest.approx(score_no)
    assert scorer_with.primitive_fire_count[prim.name] == 0
