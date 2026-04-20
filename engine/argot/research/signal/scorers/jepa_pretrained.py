from __future__ import annotations

from argot.research.signal.base import REGISTRY, SignalScorer
from argot.train import ModelBundle, train_model
from argot.validate import score_records, split_by_time


class JepaPretrainedScorer:
    name = "jepa_pretrained"

    def __init__(self) -> None:
        self._bundle: ModelBundle | None = None

    def fit(self, corpus: list[dict]) -> None:
        train_records, _ = split_by_time(corpus, ratio=0.8)
        self._bundle = train_model(train_records, encoder="pretrained", epochs=20)

    def score(self, fixtures: list[dict]) -> list[float]:
        if self._bundle is None:
            raise RuntimeError("fit() must be called before score()")
        return score_records(self._bundle, fixtures)


REGISTRY["jepa_pretrained"] = JepaPretrainedScorer
