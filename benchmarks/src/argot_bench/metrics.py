from __future__ import annotations

import math
from collections import Counter
from collections.abc import Iterable, Mapping, Sequence
from typing import Any

from sklearn.metrics import roc_auc_score


def auc_catalog(break_scores: Sequence[float], control_scores: Sequence[float]) -> float:
    """ROC-AUC with catalog breaks as positives and real-PR hunks as negatives."""
    if not break_scores or not control_scores:
        return 0.0
    y_true = [1] * len(break_scores) + [0] * len(control_scores)
    y_score = list(break_scores) + list(control_scores)
    return float(roc_auc_score(y_true, y_score))


def recall_by_category(results: Iterable[Mapping[str, Any]]) -> dict[str, float]:
    """Fraction of breaks flagged, grouped by category."""
    totals: Counter[str] = Counter()
    flagged: Counter[str] = Counter()
    for r in results:
        cat = r["category"]
        totals[cat] += 1
        if r["flagged"]:
            flagged[cat] += 1
    return {cat: flagged[cat] / totals[cat] for cat in sorted(totals)}


def fp_rate(hunks: Sequence[Mapping[str, Any]]) -> float:
    """Fraction of hunks with flagged=True."""
    if not hunks:
        return 0.0
    return sum(1 for h in hunks if h["flagged"]) / len(hunks)


def stage_attribution(results: Iterable[Mapping[str, Any]]) -> dict[str, int]:
    """Count of each scorer reason."""
    return dict(Counter(r["reason"] for r in results))


def threshold_cv(thresholds: Sequence[float]) -> float:
    """Coefficient of variation (population std / mean) over per-seed thresholds."""
    if not thresholds:
        return 0.0
    mean = sum(thresholds) / len(thresholds)
    if mean == 0.0:
        return 0.0
    var = sum((t - mean) ** 2 for t in thresholds) / len(thresholds)
    std = math.sqrt(var)
    return std / mean


def calibration_stability(
    cal_hunk_sets: Sequence[set[str]],
    thresholds: Sequence[float],
) -> dict[str, float]:
    """Pool-capped calibration stability: pairwise Jaccard on hunk IDs + relative
    variance on thresholds.

    rel_var = variance(thresholds) / mean(thresholds), divide-by-zero → 0.
    jaccard = mean of all pairwise |A∩B|/|A∪B| over cal_hunk_sets.
    """
    if not thresholds:
        return {"rel_var": 0.0, "jaccard": 0.0}

    mean = sum(thresholds) / len(thresholds)
    if mean == 0.0:
        rel_var = 0.0
    else:
        var = sum((t - mean) ** 2 for t in thresholds) / len(thresholds)
        rel_var = var / mean

    n = len(cal_hunk_sets)
    if n < 2:
        return {"rel_var": rel_var, "jaccard": 1.0 if n == 1 else 0.0}
    pairs: list[float] = []
    for i in range(n):
        for j in range(i + 1, n):
            a = cal_hunk_sets[i]
            b = cal_hunk_sets[j]
            union = a | b
            if not union:
                pairs.append(1.0)
            else:
                pairs.append(len(a & b) / len(union))
    return {"rel_var": rel_var, "jaccard": sum(pairs) / len(pairs)}
