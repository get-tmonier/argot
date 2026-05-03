"""Tests for the shape-primitive scaffolding.

Verifies:
- Empty primitive list is a true no-op (bit-identical scoring vs no list).
- A stub primitive that returns a fixed contribution actually fires
  through the dispatch in weighted_contribution_for_file.
- The registry rejects unknown names loudly.
- Per-cluster baseline is fitted only against the primitive's cluster
  files (not pooled).
- Cluster-size floor is honored (primitive abstains below
  min_cluster_size).
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import pytest

from argot.scoring.scorers.call_receiver import CallReceiverScorer
from argot.scoring.scorers.shape_primitive import Language, ShapePrimitive
from argot.scoring.scorers.shape_primitive_registry import (
    build_shape_primitives,
    register_shape_primitive,
)

# ---------------------------------------------------------------------------
# Stub primitives for testing
# ---------------------------------------------------------------------------


@dataclass
class _StubBaseline:
    n_files: int


@dataclass
class _ConstantPrimitive:
    """Returns ``contribution`` whenever the cluster has a baseline AND
    cluster_size >= min_cluster_size. Otherwise abstains."""

    name: str = "stub_constant"
    min_cluster_size: int = 0
    cluster_bonus_clip: float = 5.0
    contribution: float = 1.5
    fit_calls: list[tuple[Language, int]] = field(default_factory=list)
    score_calls: int = 0

    def fit_cluster_baseline(
        self,
        cluster_files: Iterable[tuple[Path, str]],
        language: Language,
    ) -> _StubBaseline | None:
        files = list(cluster_files)
        self.fit_calls.append((language, len(files)))
        if not files:
            return None
        return _StubBaseline(n_files=len(files))

    def score(
        self,
        hunk_content: str,
        *,
        baseline: _StubBaseline | None,
        cluster_size: int,
    ) -> float:
        self.score_calls += 1
        if baseline is None or cluster_size < self.min_cluster_size:
            return 0.0
        return self.contribution


def _make_corpus(tmp_path: Path) -> list[Path]:
    """Build a 6-file 2-cluster corpus matching the existing
    cluster-rare-attestation test fixtures."""
    files = []
    for i in range(3):
        f = tmp_path / f"math_{i}.ts"
        f.write_text("Math.floor(x); Math.random(); Math.min(a,b);")
        files.append(f)
    for i in range(3):
        f = tmp_path / f"fetch_{i}.ts"
        f.write_text("fetch(url); Promise.resolve(x); console.log('done');")
        files.append(f)
    return files


# ---------------------------------------------------------------------------
# Empty-list no-op invariant (caught-fixtures preserved by construction)
# ---------------------------------------------------------------------------


def test_empty_primitive_list_is_noop_vs_no_list(tmp_path: Path) -> None:
    """Default ``shape_primitives=None`` and empty ``[]`` must score identically."""
    files = _make_corpus(tmp_path)
    s_default = CallReceiverScorer(files, language="typescript", n_clusters=2)
    s_empty = CallReceiverScorer(files, language="typescript", n_clusters=2, shape_primitives=[])
    assert s_default.shape_primitives == []
    assert s_empty.shape_primitives == []

    hunk = "Math.random();"
    file_path = files[0]
    a = s_default.weighted_contribution_for_file(
        hunk, file_path, alpha=2.0, root_bonus=2.0, cluster_bonus=5.0, cap=10.0
    )
    b = s_empty.weighted_contribution_for_file(
        hunk, file_path, alpha=2.0, root_bonus=2.0, cluster_bonus=5.0, cap=10.0
    )
    assert a == pytest.approx(b)


# ---------------------------------------------------------------------------
# Dispatch + fire counter
# ---------------------------------------------------------------------------


def test_stub_primitive_fires_through_dispatch(tmp_path: Path) -> None:
    """A stub primitive that always returns 1.5 should add 1.5 to the score."""
    files = _make_corpus(tmp_path)
    primitive = _ConstantPrimitive(contribution=1.5, min_cluster_size=0)

    s_no_prim = CallReceiverScorer(files, language="typescript", n_clusters=2)
    s_with = CallReceiverScorer(
        files, language="typescript", n_clusters=2, shape_primitives=[primitive]
    )

    # Each primitive must have been invoked once per cluster at fit time
    # (2 clusters → 2 fit calls).
    assert len(primitive.fit_calls) == 2
    assert all(lang == "typescript" for lang, _ in primitive.fit_calls)
    # Both clusters got non-zero baselines (3 files each).
    assert primitive.name in s_with.primitive_baselines
    assert all(bl is not None for bl in s_with.primitive_baselines[primitive.name].values())

    hunk = "Math.random();"
    file_path = files[0]
    base = s_no_prim.weighted_contribution_for_file(
        hunk, file_path, alpha=2.0, root_bonus=2.0, cluster_bonus=5.0, cap=10.0
    )
    treated = s_with.weighted_contribution_for_file(
        hunk, file_path, alpha=2.0, root_bonus=2.0, cluster_bonus=5.0, cap=10.0
    )
    assert treated == pytest.approx(base + 1.5)
    assert s_with.primitive_fire_count[primitive.name] == 1


def test_primitive_score_clipped_at_cap(tmp_path: Path) -> None:
    """Primitive contribution + existing weights are clipped at ``cap``."""
    files = _make_corpus(tmp_path)
    # Primitive contributes 100 — way over cap=5.0.
    primitive = _ConstantPrimitive(contribution=100.0)
    s = CallReceiverScorer(files, language="typescript", n_clusters=2, shape_primitives=[primitive])
    score = s.weighted_contribution_for_file(
        "Math.random();",
        files[0],
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=5.0,
        cap=5.0,
    )
    assert score == pytest.approx(5.0)


# ---------------------------------------------------------------------------
# Cluster-size floor
# ---------------------------------------------------------------------------


def test_primitive_abstains_below_min_cluster_size(tmp_path: Path) -> None:
    """When cluster_size < min_cluster_size, the primitive returns 0."""
    files = _make_corpus(tmp_path)
    # Each cluster has 3 files; min_cluster_size=10 → all clusters abstain.
    primitive = _ConstantPrimitive(contribution=1.5, min_cluster_size=10)
    s = CallReceiverScorer(files, language="typescript", n_clusters=2, shape_primitives=[primitive])
    score = s.weighted_contribution_for_file(
        "Math.random();",
        files[0],
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=5.0,
        cap=10.0,
    )
    # Score must equal the no-primitive baseline.
    s_no = CallReceiverScorer(files, language="typescript", n_clusters=2)
    base = s_no.weighted_contribution_for_file(
        "Math.random();",
        files[0],
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=5.0,
        cap=10.0,
    )
    assert score == pytest.approx(base)
    assert s.primitive_fire_count[primitive.name] == 0


# ---------------------------------------------------------------------------
# Per-cluster baseline isolation
# ---------------------------------------------------------------------------


def test_baselines_are_per_cluster_not_pooled(tmp_path: Path) -> None:
    """fit_cluster_baseline must see ONLY the files in its target cluster."""
    files = _make_corpus(tmp_path)
    primitive = _ConstantPrimitive()
    CallReceiverScorer(files, language="typescript", n_clusters=2, shape_primitives=[primitive])
    # 2 clusters × 3 files each = 2 fit calls, each seeing 3 files.
    assert len(primitive.fit_calls) == 2
    assert all(n_files == 3 for _, n_files in primitive.fit_calls)


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


def test_registry_rejects_unknown_name() -> None:
    with pytest.raises(KeyError, match="unknown shape primitive"):
        build_shape_primitives(["not_a_real_primitive"])


def test_registry_round_trip() -> None:
    """A registered name resolves to a fresh instance from the factory."""
    counter = {"calls": 0}

    def _factory() -> ShapePrimitive[Any]:
        counter["calls"] += 1
        return _ConstantPrimitive(name="round_trip", contribution=2.0)

    register_shape_primitive("round_trip", _factory)
    out = build_shape_primitives(["round_trip", "round_trip"])
    assert len(out) == 2
    assert all(p.name == "round_trip" for p in out)
    assert counter["calls"] == 2


def test_registry_idempotent_re_registration() -> None:
    """Re-registering with the SAME factory is allowed (e.g. test re-import)."""

    def _factory() -> ShapePrimitive[Any]:
        return _ConstantPrimitive(name="idempotent")

    register_shape_primitive("idempotent", _factory)
    register_shape_primitive("idempotent", _factory)  # no error


def test_registry_rejects_double_registration_with_different_factory() -> None:
    """Registering the same name with a different factory is an error."""

    def _factory_a() -> ShapePrimitive[Any]:
        return _ConstantPrimitive(name="conflict")

    def _factory_b() -> ShapePrimitive[Any]:
        return _ConstantPrimitive(name="conflict")

    register_shape_primitive("conflict", _factory_a)
    with pytest.raises(ValueError, match="already registered"):
        register_shape_primitive("conflict", _factory_b)


# ---------------------------------------------------------------------------
# Single-cluster mode (no clustering) — primitives must be no-op
# ---------------------------------------------------------------------------


def test_primitive_no_op_when_n_clusters_is_one(tmp_path: Path) -> None:
    """With n_clusters=1, no per-cluster baselines exist, so primitives
    must return their abstain value (cluster_id is None at score time)."""
    files = _make_corpus(tmp_path)
    primitive = _ConstantPrimitive(contribution=1.5)
    s = CallReceiverScorer(files, language="typescript", n_clusters=1, shape_primitives=[primitive])
    # No clusters built → no fit calls.
    assert primitive.fit_calls == []
    # Score-time path: weighted_contribution_for_file with n_clusters=1
    # has cluster_id=None, so the primitive dispatch loop is skipped.
    score = s.weighted_contribution_for_file(
        "Math.random();",
        files[0],
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=5.0,
        cap=10.0,
    )
    s_no_prim = CallReceiverScorer(files, language="typescript", n_clusters=1)
    base = s_no_prim.weighted_contribution_for_file(
        "Math.random();",
        files[0],
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=5.0,
        cap=10.0,
    )
    assert score == pytest.approx(base)
    assert s.primitive_fire_count[primitive.name] == 0
