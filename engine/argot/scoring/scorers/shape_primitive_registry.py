"""Shape-primitive factory registry.

Maps primitive names (as passed on the bench / scorer-config command
line) to factory callables that return concrete ``ShapePrimitive``
instances. New primitives register here when they land:

    from argot.scoring.scorers.except_return_raise_ratio import ExceptReturnRaiseRatio
    register_shape_primitive("except_return_raise_ratio", ExceptReturnRaiseRatio)

Callers translate a list of names → instances via
``build_shape_primitives``. Unknown names raise ``KeyError`` so a
typo'd CLI flag fails loudly.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from argot.scoring.scorers.shape_primitive import ShapePrimitive

ShapePrimitiveFactory = Callable[[], ShapePrimitive[Any]]

# Populated by primitive modules at import time (via
# ``shape_primitive_registrations``).
_REGISTRY: dict[str, ShapePrimitiveFactory] = {}


def register_shape_primitive(name: str, factory: ShapePrimitiveFactory) -> None:
    """Register *factory* under *name*. Idempotent (re-registering the
    same name with the same factory is allowed, e.g. re-import in tests)."""
    if name in _REGISTRY and _REGISTRY[name] is not factory:
        raise ValueError(f"shape-primitive {name!r} already registered with a different factory")
    _REGISTRY[name] = factory


def known_shape_primitives() -> list[str]:
    """Return registered primitive names in alphabetical order."""
    return sorted(_REGISTRY)


def build_shape_primitives(names: list[str]) -> list[ShapePrimitive[Any]]:
    """Translate a list of registered names into freshly-built primitive
    instances. Unknown names raise ``KeyError`` (no silent skip)."""
    out: list[ShapePrimitive[Any]] = []
    for name in names:
        try:
            factory = _REGISTRY[name]
        except KeyError as exc:
            known = ", ".join(known_shape_primitives()) or "<none>"
            raise KeyError(f"unknown shape primitive {name!r}; known: {known}") from exc
        out.append(factory())
    return out


__all__ = [
    "ShapePrimitiveFactory",
    "build_shape_primitives",
    "known_shape_primitives",
    "register_shape_primitive",
]
