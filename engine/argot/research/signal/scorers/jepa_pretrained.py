from __future__ import annotations

import random
from typing import Any

import numpy as np
import torch

from argot.research.signal.base import REGISTRY
from argot.train import ModelBundle, train_model
from argot.validate import score_records, split_by_time


class JepaPretrainedScorer:
    name = "jepa_pretrained"

    def __init__(
        self,
        *,
        epochs: int = 20,
        lr: float = 5e-5,
        batch_size: int = 128,
        lambd: float = 0.09,
        random_seed: int | None = None,
    ) -> None:
        self._epochs = epochs
        self._lr = lr
        self._batch_size = batch_size
        self._lambd = lambd
        self._random_seed = random_seed
        self._bundle: ModelBundle | None = None

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        if self._random_seed is not None:
            torch.manual_seed(self._random_seed)
            np.random.seed(self._random_seed)
            random.seed(self._random_seed)
        train_records, _ = split_by_time(corpus, ratio=0.8)
        self._bundle = train_model(
            train_records,
            encoder="pretrained",
            epochs=self._epochs,
            lr=self._lr,
            batch_size=self._batch_size,
            lambd=self._lambd,
        )

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        if self._bundle is None:
            raise RuntimeError("fit() must be called before score()")
        return score_records(self._bundle, fixtures)


REGISTRY["jepa_pretrained"] = JepaPretrainedScorer
