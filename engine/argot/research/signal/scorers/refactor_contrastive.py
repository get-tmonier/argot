from __future__ import annotations

from pathlib import Path
from typing import Any

from argot.research.signal.base import REGISTRY
from argot.research.signal.scorers.jepa_infonce import JepaInfoNCEScorer
from argot.research.signal.scorers.refactor_miner import mine_refactor_pairs

_MIN_PAIRS = 10


def _text_to_record(text: str, index: int) -> dict[str, Any]:
    """Convert raw text into a synthetic corpus record compatible with JepaInfoNCEScorer."""
    return {
        "hunk_tokens": [{"text": tok} for tok in text.split()],
        "context_before": [],
        "context_after": [],
        "author_date_iso": str(index),
        "language": "python",
    }


class RefactorContrastiveScorer:
    """Contrastive scorer trained on refactor-mined before/after pairs from git history.

    Strategy
    --------
    1. Mine the target repo's git log for refactor-style commits.
    2. If >= 10 pairs found, train JepaInfoNCEScorer on the *after* (idiomatic) texts.
    3. If < 10 pairs found, fall back to training on the supplied corpus (same as
       plain JepaInfoNCEScorer).
    4. At score time, delegate to the inner scorer: higher score = more anomalous.
    """

    name = "refactor_contrastive"

    def __init__(
        self,
        repo_path: Path | None = None,
        *,
        epochs: int = 20,
        lr: float = 1e-4,
        tau: float = 0.07,
        beta: float = 0.1,
    ) -> None:
        self._repo_path = repo_path
        self._epochs = epochs
        self._lr = lr
        self._tau = tau
        self._beta = beta
        self._inner: JepaInfoNCEScorer | None = None

    def _make_inner(self) -> JepaInfoNCEScorer:
        return JepaInfoNCEScorer(
            epochs=self._epochs,
            lr=self._lr,
            tau=self._tau,
            beta=self._beta,
        )

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        """Train on mined after-texts (idiomatic examples) or fall back to corpus."""
        repo_path = self._repo_path if self._repo_path is not None else Path.cwd()

        pairs = mine_refactor_pairs(repo_path)

        inner = self._make_inner()

        if len(pairs) < _MIN_PAIRS:
            print(
                f"  [refactor_contrastive] only {len(pairs)} pairs mined "
                f"(< {_MIN_PAIRS}), falling back to corpus training",
                flush=True,
            )
            inner.fit(corpus)
        else:
            print(
                f"  [refactor_contrastive] {len(pairs)} pairs mined — "
                "training on after-texts (idiomatic examples)",
                flush=True,
            )
            synthetic_corpus = [
                _text_to_record(after_text, i) for i, (_before, after_text) in enumerate(pairs)
            ]
            inner.fit(synthetic_corpus)

        self._inner = inner

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        if self._inner is None:
            raise RuntimeError("fit() must be called before score()")
        return self._inner.score(fixtures)


REGISTRY["refactor_contrastive"] = RefactorContrastiveScorer
