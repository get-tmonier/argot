"""BlendScorer — convex combination of N scorers with z-scored outputs.

α are constrained to the probability simplex (sum=1, all ≥ 0).
"""

from __future__ import annotations

import statistics
from typing import Any

from argot.research.signal.base import SignalScorer


class BlendScorer:
    """Convex combination of N scorers with z-scored outputs and learned α weights.

    α are constrained to the probability simplex (sum=1, all ≥ 0).
    """

    name = "blend"

    def __init__(
        self,
        scorers: list[SignalScorer],
        alphas: list[float],
    ) -> None:
        """
        scorers: the constituent SignalScorer instances (already fitted)
        alphas: convex weights summing to 1.0, one per scorer
        """
        if len(scorers) != len(alphas):
            raise ValueError(
                f"scorers and alphas must have the same length, "
                f"got {len(scorers)} and {len(alphas)}"
            )
        self._scorers = scorers
        self._alphas = alphas
        # (mean, std) per scorer — populated by fit()
        self._stats: list[tuple[float, float]] = []

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        """Compute per-scorer z-score stats from corpus scores (for normalization at score time)."""
        # Use up to 500 corpus records to keep things fast
        sample = corpus[:500]
        self._stats = []
        for scorer in self._scorers:
            raw = scorer.score(sample)
            mean = statistics.mean(raw) if raw else 0.0
            std = statistics.stdev(raw) if len(raw) >= 2 else 1.0
            if std == 0.0:
                std = 1.0
            self._stats.append((mean, std))

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        """Z-score each scorer's output then blend with alphas."""
        if not self._stats:
            raise RuntimeError("fit() must be called before score()")

        n = len(fixtures)
        # Accumulate weighted z-scores
        blended = [0.0] * n
        for scorer, alpha, (mean, std) in zip(
            self._scorers, self._alphas, self._stats, strict=True
        ):
            raw = scorer.score(fixtures)
            for i, s in enumerate(raw):
                blended[i] += alpha * (s - mean) / std

        return blended
