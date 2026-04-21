"""Tests for RefactorContrastiveScorer — all heavy model loading is mocked."""

from __future__ import annotations

from pathlib import Path
from typing import Any
from unittest.mock import MagicMock, patch

import pytest

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_MODULE = "argot.research.signal.scorers.refactor_contrastive"


def _make_corpus(n: int = 20) -> list[dict[str, Any]]:
    """Build synthetic corpus records."""
    return [
        {
            "hunk_tokens": [{"text": f"token_{i}"}],
            "context_before": [],
            "context_after": [],
            "author_date_iso": str(i),
            "language": "python",
        }
        for i in range(n)
    ]


def _make_pairs(n: int) -> list[tuple[str, str]]:
    """Build synthetic (before, after) pairs."""
    return [(f"old code line {i}", f"new code line {i}") for i in range(n)]


def _make_mock_inner(n_fixtures: int = 5) -> MagicMock:
    """Create a mock JepaInfoNCEScorer."""
    inner = MagicMock()
    inner.score.return_value = [float(i) for i in range(n_fixtures)]
    return inner


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_fallback_to_corpus_when_few_pairs() -> None:
    """When mine_refactor_pairs returns < 10 pairs, scorer trains on the corpus."""
    from argot.research.signal.scorers.refactor_contrastive import RefactorContrastiveScorer

    corpus = _make_corpus(20)
    few_pairs = _make_pairs(5)  # below threshold

    mock_inner = _make_mock_inner()

    with (
        patch(f"{_MODULE}.mine_refactor_pairs", return_value=few_pairs) as mock_mine,
        patch(f"{_MODULE}.JepaInfoNCEScorer", return_value=mock_inner) as mock_cls,
    ):
        scorer = RefactorContrastiveScorer(repo_path=Path("/fake/repo"))
        scorer.fit(corpus)

    mock_mine.assert_called_once()
    mock_cls.assert_called_once()
    # fit() should have been called with the original corpus
    mock_inner.fit.assert_called_once_with(corpus)


def test_trains_on_after_texts_when_many_pairs() -> None:
    """When mine_refactor_pairs returns >= 10 pairs, scorer trains on after-texts corpus."""
    from argot.research.signal.scorers.refactor_contrastive import RefactorContrastiveScorer

    corpus = _make_corpus(20)
    many_pairs = _make_pairs(15)  # above threshold

    mock_inner = _make_mock_inner()

    with (
        patch(f"{_MODULE}.mine_refactor_pairs", return_value=many_pairs) as mock_mine,
        patch(f"{_MODULE}.JepaInfoNCEScorer", return_value=mock_inner) as mock_cls,
    ):
        scorer = RefactorContrastiveScorer(repo_path=Path("/fake/repo"))
        scorer.fit(corpus)

    mock_mine.assert_called_once()
    mock_cls.assert_called_once()

    # fit() should have been called with a synthetic corpus of 15 records
    call_args = mock_inner.fit.call_args
    synthetic_corpus: list[dict[str, Any]] = call_args[0][0]
    assert len(synthetic_corpus) == 15
    # Each record should contain "hunk_tokens" built from the after-text
    for record in synthetic_corpus:
        assert "hunk_tokens" in record
        assert record["context_before"] == []
        assert "author_date_iso" in record


def test_score_returns_correct_count() -> None:
    """score() returns the same number of floats as fixtures supplied."""
    from argot.research.signal.scorers.refactor_contrastive import RefactorContrastiveScorer

    n_fixtures = 7
    corpus = _make_corpus(20)
    fixtures = _make_corpus(n_fixtures)

    mock_inner = _make_mock_inner(n_fixtures)

    with (
        patch(f"{_MODULE}.mine_refactor_pairs", return_value=_make_pairs(15)),
        patch(f"{_MODULE}.JepaInfoNCEScorer", return_value=mock_inner),
    ):
        scorer = RefactorContrastiveScorer(repo_path=Path("/fake/repo"))
        scorer.fit(corpus)
        scores = scorer.score(fixtures)

    assert len(scores) == n_fixtures
    assert all(isinstance(s, float) for s in scores)


def test_score_before_fit_raises() -> None:
    """Calling score() before fit() raises RuntimeError."""
    from argot.research.signal.scorers.refactor_contrastive import RefactorContrastiveScorer

    scorer = RefactorContrastiveScorer()
    with pytest.raises(RuntimeError, match="fit\\(\\)"):
        scorer.score([])


def test_defaults_to_cwd_when_repo_path_none() -> None:
    """When repo_path=None, mine_refactor_pairs is called with Path.cwd()."""
    from argot.research.signal.scorers.refactor_contrastive import RefactorContrastiveScorer

    corpus = _make_corpus(5)
    mock_inner = _make_mock_inner()

    with (
        patch(f"{_MODULE}.mine_refactor_pairs", return_value=[]) as mock_mine,
        patch(f"{_MODULE}.JepaInfoNCEScorer", return_value=mock_inner),
    ):
        scorer = RefactorContrastiveScorer(repo_path=None)
        scorer.fit(corpus)

    called_path = mock_mine.call_args[0][0]
    assert called_path == Path.cwd()


def test_registry_contains_scorer() -> None:
    """refactor_contrastive must be present in REGISTRY after import."""
    from argot.research.signal.base import REGISTRY
    from argot.research.signal.scorers import refactor_contrastive  # noqa: F401

    assert "refactor_contrastive" in REGISTRY
