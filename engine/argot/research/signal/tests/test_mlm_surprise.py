"""Tests for MlmSurpriseScorer — all model loading is mocked out."""

from __future__ import annotations

from typing import Any
from unittest.mock import MagicMock

import torch

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_VOCAB_SIZE = 32
_SEQ_LEN = 8
_MASK_TOKEN_ID = 99


def _make_scorer(variant: str) -> Any:
    """Build a MlmSurpriseScorer with all heavy model deps mocked out."""
    from typing import Literal

    from argot.research.signal.scorers.mlm_surprise import MlmSurpriseScorer

    lit_variant: Literal["mean", "min", "p05"] = variant  # type: ignore[assignment]
    scorer = MlmSurpriseScorer.__new__(MlmSurpriseScorer)
    scorer._variant = lit_variant
    scorer._batch_size = 32
    scorer.name = f"mlm_surprise_{variant}"
    scorer._device = torch.device("cpu")
    scorer._mask_token_id = _MASK_TOKEN_ID

    # Fake tokenizer that returns deterministic fixed-length sequences
    tokenizer = MagicMock()
    # full text encoding: seq of [101, 1, 2, 3, 4, 102] (CLS, tokens, SEP)
    tokenizer.return_value = {
        "input_ids": torch.tensor([[101, 1, 2, 3, 4, 102]]),
        "attention_mask": torch.tensor([[1, 1, 1, 1, 1, 1]]),
    }
    # ctx encoding for boundary detection: [101, 1, 102] → 1 real ctx token
    tokenizer.mask_token_id = _MASK_TOKEN_ID
    scorer._tokenizer = tokenizer

    # Fake MLM model: returns uniform logits so every token has the same log-prob
    def fake_model(input_ids: torch.Tensor, attention_mask: torch.Tensor) -> Any:  # noqa: ANN401
        batch = input_ids.shape[0]
        seq = input_ids.shape[1]
        logits = torch.zeros(batch, seq, _VOCAB_SIZE)
        result = MagicMock()
        result.logits = logits
        return result

    scorer._model = fake_model
    return scorer


def _make_fixture(n_hunk: int = 3, with_ctx: bool = False) -> dict[str, Any]:
    hunk_tokens = [{"text": f"tok{i}"} for i in range(n_hunk)]
    result: dict[str, Any] = {"hunk_tokens": hunk_tokens}
    if with_ctx:
        result["ctx_before_tokens"] = [{"text": "ctx_tok"}]
    return result


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_fit_is_noop() -> None:
    """fit() should not raise and should require no corpus."""
    scorer = _make_scorer("mean")
    scorer.fit([])
    scorer.fit([{"hunk_tokens": [{"text": "x"}]}])


def test_score_returns_correct_count() -> None:
    """score() should return exactly one float per fixture."""
    scorer = _make_scorer("mean")

    # Patch _score_one so we don't need a real tokenizer call chain
    scorer._score_one = lambda rec: 1.0

    fixtures = [_make_fixture(), _make_fixture(), _make_fixture()]
    scores = scorer.score(fixtures)

    assert len(scores) == 3
    assert all(isinstance(s, float) for s in scores)


def test_variants_aggregate_differently() -> None:
    """The three variants should (in general) produce different values from the same logprobs."""
    # Build three scorers using the mocked helpers
    mean_scorer = _make_scorer("mean")
    min_scorer = _make_scorer("min")
    p05_scorer = _make_scorer("p05")

    # Inject known logprobs to verify aggregation formulae
    # log-probs: mixed values so the three variants differ
    logprobs = [-0.5, -1.0, -2.0, -0.1, -5.0]

    mean_val = mean_scorer._aggregate(logprobs)
    min_val = min_scorer._aggregate(logprobs)
    p05_val = p05_scorer._aggregate(logprobs)

    # mean of negated: (0.5 + 1.0 + 2.0 + 0.1 + 5.0) / 5 = 1.72
    assert abs(mean_val - 1.72) < 1e-6, f"mean={mean_val}"

    # min of negated = max(0.5, 1.0, 2.0, 0.1, 5.0) = 5.0
    assert abs(min_val - 5.0) < 1e-6, f"min={min_val}"

    # p05: 5th-percentile of sorted logprobs = sorted[-5.0, -2.0, -1.0, -0.5, -0.1][0] = -5.0
    # negate → 5.0
    assert abs(p05_val - 5.0) < 1e-6, f"p05={p05_val}"

    # They are not all identical for this input
    # (mean_val != min_val is true here)
    assert mean_val != min_val


def test_score_empty_hunk_positions() -> None:
    """Edge case: a fixture that produces no hunk positions returns 0.0."""
    scorer = _make_scorer("mean")

    # Force hunk_positions to be empty by returning a very short sequence
    short_tokenizer = MagicMock()
    short_tokenizer.return_value = {
        "input_ids": torch.tensor([[101, 102]]),  # CLS + SEP only
        "attention_mask": torch.tensor([[1, 1]]),
    }
    short_tokenizer.mask_token_id = _MASK_TOKEN_ID
    scorer._tokenizer = short_tokenizer

    result = scorer._score_one({"hunk_tokens": [{"text": "x"}]})
    assert result == 0.0


def test_registry_has_three_variants() -> None:
    """All three variants should be registered in REGISTRY."""
    import argot.research.signal.scorers.mlm_surprise  # noqa: F401
    from argot.research.signal.base import REGISTRY

    assert "mlm_surprise_mean" in REGISTRY
    assert "mlm_surprise_min" in REGISTRY
    assert "mlm_surprise_p05" in REGISTRY


def test_compute_logprobs_batching() -> None:
    """_compute_logprobs should work correctly with batch_size < n_positions."""
    scorer = _make_scorer("mean")
    scorer._batch_size = 2  # force multiple batches

    # input_ids: [CLS, tok1, tok2, tok3, SEP]
    input_ids = torch.tensor([101, 10, 20, 30, 102])
    attention_mask = torch.tensor([1, 1, 1, 1, 1])
    hunk_positions = [1, 2, 3]

    logprobs = scorer._compute_logprobs(input_ids, attention_mask, hunk_positions)

    assert len(logprobs) == 3
    assert all(isinstance(lp, float) for lp in logprobs)
    # Uniform logits → log_softmax gives -log(vocab_size) for all tokens
    expected = -torch.log(torch.tensor(float(_VOCAB_SIZE))).item()
    for lp in logprobs:
        assert abs(lp - expected) < 1e-5, f"lp={lp} expected={expected}"
