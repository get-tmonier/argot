from __future__ import annotations

from typing import Any

import numpy as np


def percentile_rank(value: float, distribution: list[float]) -> float:
    arr = np.array(distribution)
    return float(np.mean(arr < value) * 100)


def _score_to_tag(score: float, threshold: float) -> str:
    if score <= threshold + 0.3:
        return "unusual"
    elif score <= threshold + 0.6:
        return "suspicious"
    else:
        return "foreign"


def select_style_examples(records: list[dict[str, Any]], *, n: int = 5) -> list[dict[str, Any]]:
    """Pick n lowest-surprise records, one per file where possible."""
    sorted_records = sorted(records, key=lambda r: r["_score"])
    seen_files: set[str] = set()
    diverse: list[dict[str, Any]] = []
    remainder: list[dict[str, Any]] = []

    for r in sorted_records:
        fp = r.get("file_path", "")
        if fp not in seen_files:
            seen_files.add(fp)
            diverse.append(r)
        else:
            remainder.append(r)
        if len(diverse) >= n:
            break

    result = diverse[:n]
    if len(result) < n:
        result += remainder[: n - len(result)]
    return result
