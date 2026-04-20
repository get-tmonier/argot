from __future__ import annotations

from typing import Any

import torch
import torch.nn.functional as F  # noqa: N812
from sklearn.neighbors import LocalOutlierFactor  # type: ignore[import-untyped]

from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.signal.base import REGISTRY


class LofEmbeddingScorer:
    name = "lof_embedding"

    def __init__(self) -> None:
        device = select_device()
        self._encoder: PretrainedEncoder = PretrainedEncoder(device=device)
        self._lof: LocalOutlierFactor | None = None

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in corpus]
        with torch.no_grad():
            emb = self._encoder.encode_texts(texts)
        x = F.normalize(emb, p=2, dim=1).cpu().numpy()
        n = x.shape[0]
        self._lof = LocalOutlierFactor(n_neighbors=min(20, n - 1), metric="cosine", novelty=True)
        self._lof.fit(x)

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        if self._lof is None:
            raise RuntimeError("fit() must be called before score()")
        texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in fixtures]
        with torch.no_grad():
            emb = self._encoder.encode_texts(texts)
        x = F.normalize(emb, p=2, dim=1).cpu().numpy()
        # score_samples returns negative values; negate so higher = more anomalous
        return [-float(s) for s in self._lof.score_samples(x)]


REGISTRY["lof_embedding"] = LofEmbeddingScorer
