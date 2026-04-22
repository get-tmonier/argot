# engine/argot/research/signal/phase14/fixtures/stage2_only/genexpr_shift.py
"""sum/any/all genexpr chains where the host file uses list comprehensions — stdlib only.

FastAPI corpus favours list comprehensions; heavy any()/all()/sum() generator chains
over in-memory collections are an uncommon pattern.
Imports: collections (stdlib, in FastAPI corpus).
"""
from __future__ import annotations

from collections import Counter


def analyse(records: list[dict[str, object]]) -> dict[str, object]:
    values: list[float] = [float(r["value"]) for r in records if "value" in r]  # type: ignore[arg-type]
    mean = sum(values) / len(values) if values else 0.0
    return {
        "total": sum(v for v in values),
        "positive_count": sum(1 for v in values if v > 0),
        "has_outlier": any(abs(v - mean) > 3 * mean for v in values) if values else False,
        "all_finite": all(v == v and abs(v) < 1e308 for v in values),
        "max_key": max(
            (str(r.get("key", "")) for r in records),
            key=len,
            default="",
        ),
    }


def top_n(counts: Counter[str], n: int) -> list[tuple[str, int]]:
    return sorted(
        ((k, v) for k, v in counts.items() if v > 0),
        key=lambda kv: (-kv[1], kv[0]),
    )[:n]
