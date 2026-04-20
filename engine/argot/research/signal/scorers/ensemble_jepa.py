from __future__ import annotations

from typing import Any, Literal

from argot.research.signal.base import REGISTRY
from argot.research.signal.scorers.jepa_custom import JepaCustomScorer


class EnsembleJepaScorer:
    """Inference ensemble over JepaCustomScorer(flat, depth=6, mlp_dim=1024).

    Trains N predictors with consecutive seeds [base_seed, base_seed+N-1] and
    averages per-fixture surprise scores at inference time. Reduces variance
    from lucky/unlucky initialisations while preserving the higher mean of
    flat_d6m1024 (mean=0.221 unensembled).
    """

    name = "ensemble_jepa"

    def __init__(
        self,
        *,
        n: int = 3,
        base_seed: int = 0,
        aggregation: Literal["mean", "topk", "random_topk"] = "mean",
        topk_k: int = 64,
        zscore_vs_corpus: bool = False,
    ) -> None:
        self._n = n
        self._base_seed = base_seed
        self._aggregation = aggregation
        self._topk_k = topk_k
        self._zscore_vs_corpus = zscore_vs_corpus
        self._members: list[JepaCustomScorer] = []

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        self._members = []
        for i in range(self._n):
            member = JepaCustomScorer(
                epochs=20,
                lr=1e-4,
                lr_schedule="flat",
                predictor_overrides={"depth": 6, "mlp_dim": 1024},
                random_seed=self._base_seed + i,
                aggregation=self._aggregation,
                topk_k=self._topk_k,
                zscore_vs_corpus=self._zscore_vs_corpus,
            )
            member.fit(corpus)
            self._members.append(member)

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        if not self._members:
            raise RuntimeError("fit() must be called before score()")
        all_scores = [m.score(fixtures) for m in self._members]
        return [sum(run[i] for run in all_scores) / self._n for i in range(len(fixtures))]


REGISTRY["ensemble_jepa"] = EnsembleJepaScorer
