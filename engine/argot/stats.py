from __future__ import annotations

from typing import Any

import numpy as np
from sklearn.metrics import roc_auc_score  # type: ignore[import-untyped]


def split_by_time(
    records: list[dict[str, Any]], *, ratio: float = 0.8
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    sorted_records = sorted(records, key=lambda r: int(r["author_date_iso"]))
    split_idx = int(len(sorted_records) * ratio)
    return sorted_records[:split_idx], sorted_records[split_idx:]


def compute_percentiles(scores: list[float]) -> dict[str, float]:
    arr = np.array(scores)
    return {
        "min": float(np.min(arr)),
        "p25": float(np.percentile(arr, 25)),
        "median": float(np.median(arr)),
        "p75": float(np.percentile(arr, 75)),
        "p95": float(np.percentile(arr, 95)),
        "max": float(np.max(arr)),
    }


def compute_auc(good_scores: list[float], bad_scores: list[float]) -> float:
    labels = [0] * len(good_scores) + [1] * len(bad_scores)
    scores = good_scores + bad_scores
    return float(roc_auc_score(labels, scores))
