from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Any, Protocol


@dataclass
class ScoredFixture:
    name: str
    scope: str
    is_break: bool
    score: float


class SignalScorer(Protocol):
    name: str  # stable id used in column headers

    def fit(self, corpus: list[dict[str, Any]]) -> None: ...  # noqa: E704
    def score(self, fixtures: list[dict[str, Any]]) -> list[float]: ...  # noqa: E704


REGISTRY: dict[str, Callable[[], SignalScorer]] = {}
