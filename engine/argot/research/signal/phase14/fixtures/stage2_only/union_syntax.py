# engine/argot/research/signal/phase14/fixtures/stage2_only/union_syntax.py
"""X | None / int | str union syntax throughout — Python 3.10+ modernisation, stdlib only.

Mirrors PR #14564's Python 3.9 modernisation axis: old Optional[X] replaced by X | None,
Union[A, B] by A | B, throughout type signatures and local variables.
Imports: json, pathlib (stdlib, in FastAPI corpus).
"""
from __future__ import annotations

import json
from pathlib import Path


def parse_config(source: str | Path) -> dict[str, str | int | bool | None]:
    text: str = source.read_text() if isinstance(source, Path) else source
    raw: dict[str, object] = json.loads(text)
    result: dict[str, str | int | bool | None] = {}
    for key, val in raw.items():
        result[key] = val if isinstance(val, (str, int, bool)) else None
    return result


def coerce(value: str | int | float | None, target: type) -> int | float | str | None:
    if value is None:
        return None
    try:
        return target(value)  # type: ignore[call-arg]
    except (ValueError, TypeError):
        return None


def merge_dicts(
    a: dict[str, object] | None,
    b: dict[str, object] | None,
) -> dict[str, object]:
    return {**(a or {}), **(b or {})}
