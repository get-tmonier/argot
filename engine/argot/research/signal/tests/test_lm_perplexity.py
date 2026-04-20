from __future__ import annotations

from typing import Any
from unittest.mock import MagicMock

import pytest

from argot.research.signal.scorers.lm_perplexity import LmPerplexityScorer


def test_lm_perplexity_fit_is_noop() -> None:
    scorer = LmPerplexityScorer.__new__(LmPerplexityScorer)
    scorer._model = MagicMock()
    scorer._tokenizer = MagicMock()
    scorer.fit([{"hunk_tokens": [{"text": "x"}]}])


def test_lm_perplexity_score_returns_floats() -> None:
    try:
        scorer = LmPerplexityScorer()
    except OSError:
        pytest.skip("codegen-350M-multi not cached")

    fixtures: list[dict[str, Any]] = [
        {
            "hunk_tokens": [{"text": "def"}, {"text": "foo"}, {"text": "("}],
            "ctx_before_tokens": [{"text": "import"}, {"text": "os"}],
        },
        {
            "hunk_tokens": [{"text": "return"}, {"text": "None"}],
        },
    ]
    scores = scorer.score(fixtures)
    assert len(scores) == 2
    assert all(isinstance(s, float) for s in scores)
