from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Protocol


@dataclass
class ScoredFixture:
    name: str
    scope: str
    is_break: bool
    score: float


class SignalScorer(Protocol):
    name: str  # stable id used in column headers

    def fit(self, corpus: list[dict]) -> None: ...  # noqa: E704
    def score(self, fixtures: list[dict]) -> list[float]: ...  # noqa: E704


REGISTRY: dict[str, Callable[[], SignalScorer]] = {}
