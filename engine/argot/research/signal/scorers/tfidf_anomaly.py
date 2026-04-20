from __future__ import annotations

from typing import Any

import numpy as np
from scipy.sparse import csr_matrix  # type: ignore[import-untyped]
from sklearn.feature_extraction.text import (  # type: ignore[import-untyped]
    TfidfVectorizer,
)
from sklearn.neighbors import NearestNeighbors  # type: ignore[import-untyped]

from argot.research.signal.base import REGISTRY


class TfidfAnomalyScorer:
    name = "tfidf_anomaly"

    def __init__(self) -> None:
        # bigrams capture local idioms; 5000 caps vocab for code token sequences
        self._vectorizer: TfidfVectorizer = TfidfVectorizer(ngram_range=(1, 2), max_features=5000)
        self._neighbors: NearestNeighbors | None = None
        self._corpus_tfidf: csr_matrix | None = None

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in corpus]
        self._corpus_tfidf = self._vectorizer.fit_transform(texts)
        k = min(5, len(corpus))
        self._neighbors = NearestNeighbors(n_neighbors=k, metric="cosine")
        self._neighbors.fit(self._corpus_tfidf)

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        if self._neighbors is None or self._corpus_tfidf is None:
            raise RuntimeError("fit() must be called before score()")
        texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in fixtures]
        fixture_tfidf = self._vectorizer.transform(texts)
        distances, _ = self._neighbors.kneighbors(fixture_tfidf)
        scores: list[float] = [float(np.mean(row)) for row in distances]
        return scores


REGISTRY["tfidf_anomaly"] = TfidfAnomalyScorer
