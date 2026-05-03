"""Smoke test for the shape-primitive registrations module.

After importing ``argot.scoring.scorers.shape_primitive_registrations``,
the four canonical primitives must be addressable via
``build_shape_primitives``.
"""

from __future__ import annotations

import argot.scoring.scorers.shape_primitive_registrations  # noqa: F401  (side-effect import)
from argot.scoring.scorers.shape_primitive_registry import (
    build_shape_primitives,
    known_shape_primitives,
)

_EXPECTED_PRIMITIVES = (
    "except_return_raise_ratio",
    "call_scope_fraction",
    "namespace_jsd",
    "fall_through_guards",
)


def test_all_canonical_primitives_registered() -> None:
    names = known_shape_primitives()
    for expected in _EXPECTED_PRIMITIVES:
        assert expected in names, f"missing registered primitive: {expected}"


def test_build_resolves_canonical_names() -> None:
    primitives = build_shape_primitives(list(_EXPECTED_PRIMITIVES))
    assert len(primitives) == 4
    assert [p.name for p in primitives] == list(_EXPECTED_PRIMITIVES)
    # Every primitive contractually exposes the three required attributes.
    for p in primitives:
        assert isinstance(p.name, str)
        assert isinstance(p.min_cluster_size, int)
        assert p.min_cluster_size > 0
        assert isinstance(p.cluster_bonus_clip, float)
        assert p.cluster_bonus_clip > 0
