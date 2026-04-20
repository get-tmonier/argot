from __future__ import annotations

import random
from typing import Any

import numpy as np
import torch
import torch.nn.functional as F  # noqa: N812

from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.signal.base import REGISTRY
from argot.research.signal.scorers.jepa_custom import JepaCustomScorer
from argot.train import _texts_for_records


class JepaFilteredScorer:
    """JepaCustomScorer(flat, depth=4, mlp_dim=1024) with corpus pre-filtering.

    Drops corpus records whose max cosine similarity to any break fixture exceeds
    the top-τ percentile, preventing the predictor from learning to expect
    break-style hunks during training.

    Usage (via sweep.py):
        scorer = JepaFilteredScorer(tau_percentile=1.0)
        scorer.prime_breaks(break_fixture_records)  # must be called before fit()
        scorer.fit(corpus)
        scores = scorer.score(fixtures)
    """

    name = "jepa_filtered"

    def __init__(
        self,
        *,
        tau_percentile: float = 1.0,
        random_seed: int | None = None,
    ) -> None:
        self._tau_percentile = tau_percentile
        self._random_seed = random_seed
        self._break_fixtures: list[dict[str, Any]] = []
        self._inner = JepaCustomScorer(
            epochs=20,
            lr=1e-4,
            lr_schedule="flat",
            predictor_overrides={"depth": 4, "mlp_dim": 1024},
            random_seed=random_seed,
        )

    def prime_breaks(self, break_fixtures: list[dict[str, Any]]) -> None:
        self._break_fixtures = break_fixtures

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        if self._random_seed is not None:
            torch.manual_seed(self._random_seed)
            np.random.seed(self._random_seed)
            random.seed(self._random_seed)

        filtered = self._filter(corpus)
        n_dropped = len(corpus) - len(filtered)
        print(
            f"  filter  τ={self._tau_percentile}%: dropped {n_dropped}/{len(corpus)} records",
            flush=True,
        )
        self._inner.fit(filtered)

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        return self._inner.score(fixtures)

    def _filter(self, corpus: list[dict[str, Any]]) -> list[dict[str, Any]]:
        if not self._break_fixtures:
            return corpus

        device = select_device()
        encoder = PretrainedEncoder(device=device)

        _, break_texts = _texts_for_records(self._break_fixtures)
        _, corpus_texts = _texts_for_records(corpus)

        with torch.no_grad():
            break_emb = F.normalize(encoder.encode_texts(break_texts).cpu(), p=2, dim=1)
            corpus_emb = F.normalize(encoder.encode_texts(corpus_texts).cpu(), p=2, dim=1)

        max_sim = (corpus_emb @ break_emb.T).max(dim=1).values
        threshold = float(torch.quantile(max_sim, 1.0 - self._tau_percentile / 100.0))
        return [r for r, keep in zip(corpus, (max_sim <= threshold).tolist(), strict=True) if keep]


REGISTRY["jepa_filtered"] = JepaFilteredScorer
