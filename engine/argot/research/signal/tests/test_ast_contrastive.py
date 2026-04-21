from __future__ import annotations

from collections import Counter
from typing import Any

from argot.research.signal.scorers.ast_contrastive import ContrastiveAstTreeletScorer


def _make_record(source: str, path: str = "test.py") -> dict[str, Any]:
    return {"context_before": [], "hunk_tokens": [{"text": source}], "file_path": path}


def test_smoke_empty_corpus() -> None:
    scorer = ContrastiveAstTreeletScorer()
    scorer.fit([])
    records = [_make_record("def foo(x): return x + 1")]
    scores = scorer.score(records)
    assert len(scores) == 1
    assert isinstance(scores[0], float)


def test_returns_float_after_fit() -> None:
    source = "def foo(x):\n    return x + 1"
    scorer = ContrastiveAstTreeletScorer()
    corpus = [_make_record(source)]
    scorer.fit(corpus)
    scores = scorer.score([_make_record(source)])
    assert len(scores) == 1
    assert isinstance(scores[0], float)


def test_direction_anomaly_high_when_rare_in_model_a() -> None:
    """Hunk with a treelet that is rare in the repo (model_A) but common in
    generic Python (model_B) should receive a HIGH anomaly score — i.e. it
    looks more generic than repo-specific, signalling a potential paradigm break.
    """
    treelet = "d1:AsyncFunctionDef>arguments"
    source = "async def foo(x, y): return x"

    scorer = ContrastiveAstTreeletScorer(epsilon=1.0)
    scorer._model_a = Counter({treelet: 0})   # rare in repo
    scorer._model_b = Counter({treelet: 500})  # common generically

    records = [_make_record(source)]
    scores = scorer.score(records)
    assert len(scores) == 1
    assert scores[0] > 0.0, f"Expected positive anomaly score, got {scores[0]}"


def test_direction_anomaly_low_when_common_in_model_a() -> None:
    """Hunk with a treelet that is common in the repo (model_A) should receive
    a LOW anomaly score — it looks repo-idiomatic, not a break.
    """
    treelet = "d1:AsyncFunctionDef>arguments"
    source = "async def foo(x, y): return x"

    scorer = ContrastiveAstTreeletScorer(epsilon=1.0)
    scorer._model_a = Counter({treelet: 500})  # common in repo
    scorer._model_b = Counter({treelet: 0})    # rare generically

    records = [_make_record(source)]
    scores = scorer.score(records)
    assert len(scores) == 1
    assert scores[0] < 0.0, f"Expected negative anomaly score, got {scores[0]}"


def test_minimum_treelet_returns_zero() -> None:
    scorer = ContrastiveAstTreeletScorer()
    scorer.fit([])
    # SyntaxError source → extract_treelets returns [] which is < 3
    records = [_make_record("def (")]
    scores = scorer.score(records)
    assert scores[0] == 0.0
