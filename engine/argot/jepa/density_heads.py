from __future__ import annotations

from typing import Literal

import numpy as np
from sklearn.mixture import GaussianMixture  # type: ignore[import-untyped]
from sklearn.neighbors import NearestNeighbors  # type: ignore[import-untyped]

DensityHeadKind = Literal["knn-20", "gmm-8", "gmm-16", "gmm-32"]


class DensityHead:
    """Anomaly scorer: higher score = more anomalous."""

    def fit(self, embeddings: np.ndarray) -> None:
        raise NotImplementedError

    def score(self, embeddings: np.ndarray) -> np.ndarray:
        raise NotImplementedError


class KnnHead(DensityHead):
    """Mean cosine distance to the k nearest training neighbours."""

    def __init__(self, k: int = 20) -> None:
        self.k = k
        self._nn: NearestNeighbors | None = None

    def fit(self, embeddings: np.ndarray) -> None:
        k = min(self.k, len(embeddings))
        self._nn = NearestNeighbors(n_neighbors=k, metric="cosine", algorithm="brute")
        self._nn.fit(embeddings)

    def score(self, embeddings: np.ndarray) -> np.ndarray:
        assert self._nn is not None, "call fit() first"
        dists, _ = self._nn.kneighbors(embeddings)
        return dists.mean(axis=1)  # type: ignore[no-any-return]


class GmmHead(DensityHead):
    """-log p(embedding) under a full-covariance Gaussian Mixture."""

    def __init__(self, n_components: int, seed: int = 0) -> None:
        self.n_components = n_components
        self.seed = seed
        self._gmm: GaussianMixture | None = None

    def fit(self, embeddings: np.ndarray) -> None:
        self._gmm = GaussianMixture(
            n_components=self.n_components,
            covariance_type="full",
            random_state=self.seed,
            max_iter=200,
            reg_covar=1e-3,
        )
        self._gmm.fit(embeddings.astype(np.float64))

    def score(self, embeddings: np.ndarray) -> np.ndarray:
        assert self._gmm is not None, "call fit() first"
        return -self._gmm.score_samples(embeddings.astype(np.float64))  # type: ignore[no-any-return]


def make_head(kind: DensityHeadKind, seed: int = 0) -> DensityHead:
    if kind == "knn-20":
        return KnnHead(k=20)
    elif kind == "gmm-8":
        return GmmHead(n_components=8, seed=seed)
    elif kind == "gmm-16":
        return GmmHead(n_components=16, seed=seed)
    elif kind == "gmm-32":
        return GmmHead(n_components=32, seed=seed)
    else:
        raise ValueError(f"unknown density head: {kind!r}")
