from __future__ import annotations

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
