from __future__ import annotations

import torch
import torch.nn.functional as F

from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.signal.base import REGISTRY


class KnnCosineScorer:
    name = "knn_cosine"

    def __init__(self) -> None:
        device = select_device()
        self._encoder: PretrainedEncoder = PretrainedEncoder(device=device)
        self._corpus_emb: torch.Tensor | None = None

    def fit(self, corpus: list[dict]) -> None:
        texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in corpus]
        with torch.no_grad():
            emb = self._encoder.encode_texts(texts)
        self._corpus_emb = F.normalize(emb, p=2, dim=1)

    def score(self, fixtures: list[dict]) -> list[float]:
        if self._corpus_emb is None:
            raise RuntimeError("fit() must be called before score()")
        texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in fixtures]
        with torch.no_grad():
            emb = self._encoder.encode_texts(texts)
        fixture_emb = F.normalize(emb, p=2, dim=1)
        sim = fixture_emb @ self._corpus_emb.T
        k = min(5, self._corpus_emb.shape[0])
        scores: list[float] = []
        for i in range(sim.shape[0]):
            top_k_mean = sim[i].topk(k=k).values.mean()
            scores.append(1.0 - float(top_k_mean))
        return scores


REGISTRY["knn_cosine"] = KnnCosineScorer
