"""Unit tests for AstStructuralScorer (loglik, zscore, oov variants)."""

from __future__ import annotations

import math

from argot.research.signal.scorers.ast_structural import AstStructuralScorer

# ---------------------------------------------------------------------------
# Synthetic corpus helpers
# ---------------------------------------------------------------------------

_HTTP_RECORD = {
    "_source_lines": [
        "from fastapi import HTTPException",
        "def handler():",
        "    raise HTTPException(status_code=404)",
    ]
}

_VALUE_ERROR_RECORD = {
    "_source_lines": [
        "def handler():",
        "    raise ValueError('bad')",
    ]
}

_BASE_MODEL_RECORD = {
    "_source_lines": [
        "from pydantic import BaseModel",
        "class Item(BaseModel):",
        "    name: str",
    ]
}

_DECORATOR_RECORD = {
    "_source_lines": [
        "@app.get('/items')",
        "def list_items(): pass",
    ]
}


def _make_corpus(n_http: int = 8, n_value: int = 2) -> list[dict]:  # type: ignore[type-arg]
    return [_HTTP_RECORD] * n_http + [_VALUE_ERROR_RECORD] * n_value


# ---------------------------------------------------------------------------
# loglik variant
# ---------------------------------------------------------------------------


def test_loglik_oov_exception_ranks_higher() -> None:
    corpus = _make_corpus(n_http=8, n_value=0)
    scorer = AstStructuralScorer(variant="loglik")
    scorer.fit(corpus)

    rare = {"_source_lines": ["raise RuntimeError('x')"]}
    common = {"_source_lines": ["raise HTTPException(status_code=500)"]}

    scores = scorer.score([rare, common])
    assert scores[0] > scores[1], "OOV exception should score higher (more anomalous)"


def test_loglik_empty_fixture_finite() -> None:
    scorer = AstStructuralScorer(variant="loglik")
    scorer.fit(_make_corpus())
    scores = scorer.score([{"_source_lines": []}])
    assert len(scores) == 1
    assert scores[0] == 0.0


def test_loglik_smoothing_no_blowup() -> None:
    scorer = AstStructuralScorer(variant="loglik")
    scorer.fit([_HTTP_RECORD])
    oov = {"_source_lines": ["raise MyCustomError()"]}
    scores = scorer.score([oov])
    assert math.isfinite(scores[0])
    assert scores[0] > 0


# ---------------------------------------------------------------------------
# oov variant
# ---------------------------------------------------------------------------


def test_oov_counts_unseen_features() -> None:
    corpus = _make_corpus(n_http=8, n_value=0)
    scorer = AstStructuralScorer(variant="oov")
    scorer.fit(corpus)

    oov_fix = {"_source_lines": ["raise RuntimeError('x')", "raise MyError()"]}
    known_fix = {"_source_lines": ["raise HTTPException(status_code=404)"]}

    scores = scorer.score([oov_fix, known_fix])
    assert scores[0] > scores[1]


def test_oov_unknown_decorator_raises_score() -> None:
    corpus = [_DECORATOR_RECORD] * 5
    scorer = AstStructuralScorer(variant="oov")
    scorer.fit(corpus)

    known = {"_source_lines": ["@app.get('/x')\ndef f(): pass"]}
    unknown_dec = {"_source_lines": ["@router.websocket('/ws')\ndef ws(): pass"]}

    scores = scorer.score([known, unknown_dec])
    assert scores[1] >= scores[0]


# ---------------------------------------------------------------------------
# zscore variant
# ---------------------------------------------------------------------------


def test_zscore_finite_for_all_fixtures() -> None:
    corpus = _make_corpus()
    scorer = AstStructuralScorer(variant="zscore")
    scorer.fit(corpus)
    scores = scorer.score([_HTTP_RECORD, _VALUE_ERROR_RECORD, _BASE_MODEL_RECORD])
    assert all(math.isfinite(s) for s in scores)


def test_zscore_anomalous_ranks_higher() -> None:
    corpus = _make_corpus(n_http=8, n_value=0)
    scorer = AstStructuralScorer(variant="zscore")
    scorer.fit(corpus)

    rare = {"_source_lines": ["raise RuntimeError('x')"]}
    common = {"_source_lines": ["raise HTTPException(status_code=500)"]}

    scores = scorer.score([rare, common])
    assert scores[0] > scores[1]


# ---------------------------------------------------------------------------
# Protocol compliance
# ---------------------------------------------------------------------------


def test_name_attribute_present() -> None:
    for variant in ("loglik", "zscore", "oov"):
        scorer = AstStructuralScorer(variant=variant)  # type: ignore[arg-type]
        assert isinstance(scorer.name, str)


def test_score_without_fit_returns_zeros() -> None:
    scorer = AstStructuralScorer(variant="loglik")
    scores = scorer.score([_HTTP_RECORD])
    assert len(scores) == 1


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


def test_registry_contains_all_three() -> None:
    import argot.research.signal.scorers.ast_structural  # noqa: F401
    from argot.research.signal.base import REGISTRY

    assert "ast_structural_ll" in REGISTRY
    assert "ast_structural_zscore" in REGISTRY
    assert "ast_structural_oov" in REGISTRY


def test_registry_factories_instantiate() -> None:
    import argot.research.signal.scorers.ast_structural  # noqa: F401
    from argot.research.signal.base import REGISTRY

    for key in ("ast_structural_ll", "ast_structural_zscore", "ast_structural_oov"):
        scorer = REGISTRY[key]()
        assert hasattr(scorer, "fit")
        assert hasattr(scorer, "score")
