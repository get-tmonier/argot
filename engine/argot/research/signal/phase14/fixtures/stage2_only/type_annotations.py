# engine/argot/research/signal/phase14/fixtures/stage2_only/type_annotations.py
"""PEP 695 type-parameter syntax (Python 3.12+) and Protocol annotations — stdlib only.

def f[T](...) / class C[T] generic syntax is Python 3.12; FastAPI corpus predates it.
Imports: typing (stdlib, in FastAPI corpus).
"""
from __future__ import annotations

from typing import Protocol, runtime_checkable


@runtime_checkable
class Serialisable(Protocol):
    def to_dict(self) -> dict[str, object]: ...

    def validate(self) -> bool: ...


def transform[T: Serialisable](  # type: ignore[syntax]
    items: list[T],
    filter_fn: object = None,
) -> list[dict[str, object]]:
    return [
        item.to_dict()
        for item in items
        if filter_fn is None or filter_fn(item)  # type: ignore[operator]
    ]


def merge[T](left: list[T], right: list[T], key: object) -> list[T]:  # type: ignore[syntax]
    seen: set[object] = set()
    result: list[T] = []
    for item in left + right:
        k = key(item)  # type: ignore[operator]
        if k not in seen:
            seen.add(k)
            result.append(item)
    return result
