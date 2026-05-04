"""Shape-primitive registrations.

Importing this module registers the five AST-shape primitives
(except_return_raise_ratio, call_scope_fraction, namespace_jsd,
fall_through_guards, typical_call_density) under their canonical names
with the shape-primitive registry, making them addressable from the bench /
scorer-config CLI via ``--enable-shape-primitives``.

Each registration is a thin factory that constructs a fresh primitive
instance per call, so multi-seed calibration in
``calibrate_multi_seed`` gets independent baseline state per scorer
build.

Importing this module has the side effect of populating the registry.
The bench's ``build_scorer`` ensures it is imported before any
``build_shape_primitives`` call.
"""

from __future__ import annotations

from argot.scoring.scorers.call_scope_fraction import CallScopeFraction
from argot.scoring.scorers.except_return_raise_ratio import ExceptReturnRaiseRatio
from argot.scoring.scorers.fall_through_guards import FallThroughGuards
from argot.scoring.scorers.namespace_jsd import NamespaceJsd
from argot.scoring.scorers.shape_primitive_registry import register_shape_primitive
from argot.scoring.scorers.typical_call_density import TypicalCallDensity

register_shape_primitive("except_return_raise_ratio", ExceptReturnRaiseRatio)
register_shape_primitive("call_scope_fraction", CallScopeFraction)
register_shape_primitive("namespace_jsd", NamespaceJsd)
register_shape_primitive("fall_through_guards", FallThroughGuards)
register_shape_primitive("typical_call_density", TypicalCallDensity)
