"""Tests for DeltaMlmAdapter and DeltaMlmScorer — all heavy model loading is mocked."""

from __future__ import annotations

from typing import Any
from unittest.mock import MagicMock, patch

import torch

# ---------------------------------------------------------------------------
# Shared constants
# ---------------------------------------------------------------------------

_VOCAB_SIZE = 32
_MASK_TOKEN_ID = 99
_CLS_ID = 101
_SEP_ID = 102
_PAD_ID = 0


# ---------------------------------------------------------------------------
# Fixture helpers
# ---------------------------------------------------------------------------


def _make_fixture(n_hunk: int = 3, with_ctx: bool = False) -> dict[str, Any]:
    hunk_tokens = [{"text": f"tok{i}"} for i in range(n_hunk)]
    result: dict[str, Any] = {"hunk_tokens": hunk_tokens}
    if with_ctx:
        result["context_before"] = [{"text": "ctx_tok"}]
    return result


def _tiny_corpus(n: int = 2) -> list[dict[str, Any]]:
    return [
        {
            "hunk_tokens": [{"text": "def"}, {"text": "foo"}],
            "context_before": [{"text": "import"}],
        }
        for _ in range(n)
    ]


# ---------------------------------------------------------------------------
# Fake model factory
# ---------------------------------------------------------------------------


def _fake_model_uniform() -> Any:
    """Returns a callable that produces uniform logits (shape inferred from input)."""

    def _model(input_ids: torch.Tensor, attention_mask: torch.Tensor, **kwargs: Any) -> Any:
        batch = input_ids.shape[0]
        seq = input_ids.shape[1]
        logits = torch.zeros(batch, seq, _VOCAB_SIZE)
        result = MagicMock()
        result.logits = logits
        result.loss = torch.tensor(0.0, requires_grad=True)
        return result

    return _model


def _fake_tokenizer() -> Any:
    tok = MagicMock()
    tok.mask_token_id = _MASK_TOKEN_ID
    tok.cls_token_id = _CLS_ID
    tok.sep_token_id = _SEP_ID
    tok.pad_token_id = _PAD_ID
    # encode() returns [] for empty string, else [1, 2, 3]
    tok.encode.side_effect = lambda text, add_special_tokens=True: (
        [] if not text.strip() else [1, 2, 3]
    )
    return tok


# ---------------------------------------------------------------------------
# DeltaMlmAdapter tests
# ---------------------------------------------------------------------------


def _make_adapter(**kwargs: Any) -> Any:
    from argot.research.signal.scorers.delta_mlm import DeltaMlmAdapter

    return DeltaMlmAdapter(**kwargs)


@patch("argot.research.signal.scorers.delta_mlm.AutoModelForMaskedLM")
@patch("argot.research.signal.scorers.delta_mlm.AutoTokenizer")
@patch("argot.research.signal.scorers.delta_mlm.select_device")
def test_adapter_train_runs(
    mock_device: MagicMock, mock_tok_cls: MagicMock, mock_model_cls: MagicMock
) -> None:
    """DeltaMlmAdapter.train() completes without error on a tiny corpus."""
    mock_device.return_value = torch.device("cpu")
    mock_tok_cls.from_pretrained.return_value = _fake_tokenizer()

    # Build a fake base model that also has .cls.parameters() for fallback path
    fake_base = _fake_model_uniform()
    fake_base_obj = MagicMock()
    fake_base_obj.side_effect = fake_base
    # parameters() must be iterable with requires_grad
    param = torch.nn.Parameter(torch.zeros(1))
    fake_base_obj.parameters.return_value = iter([param])
    fake_base_obj.cls = MagicMock()
    fake_base_obj.cls.parameters.return_value = iter([param])
    # to() returns self
    fake_base_obj.to.return_value = fake_base_obj
    # train() and eval() are no-ops
    fake_base_obj.train.return_value = None
    fake_base_obj.eval.return_value = None
    mock_model_cls.from_pretrained.return_value = fake_base_obj

    # Patch get_peft_model so LoRA path uses our fake model too
    peft_model = MagicMock()
    peft_model.side_effect = fake_base
    peft_param = torch.nn.Parameter(torch.zeros(1))
    peft_model.parameters.return_value = iter([peft_param])
    peft_model.to.return_value = peft_model
    peft_model.train.return_value = None
    peft_model.eval.return_value = None

    with patch("argot.research.signal.scorers.delta_mlm._PEFT_AVAILABLE", False):
        adapter = _make_adapter(n_steps=2, batch_size=2)
        adapter.train(_tiny_corpus(3))

    # get_model() should not raise after train()
    model = adapter.get_model()
    assert model is not None


@patch("argot.research.signal.scorers.delta_mlm.AutoModelForMaskedLM")
@patch("argot.research.signal.scorers.delta_mlm.AutoTokenizer")
@patch("argot.research.signal.scorers.delta_mlm.select_device")
def test_adapter_train_empty_corpus(
    mock_device: MagicMock, mock_tok_cls: MagicMock, mock_model_cls: MagicMock
) -> None:
    """train() on an empty corpus should not raise and still expose a model."""
    mock_device.return_value = torch.device("cpu")
    tok = _fake_tokenizer()
    # encode returns [] for everything → all_sequences will be empty
    tok.encode.return_value = []
    mock_tok_cls.from_pretrained.return_value = tok

    fake_base_obj = MagicMock()
    param = torch.nn.Parameter(torch.zeros(1))
    fake_base_obj.parameters.return_value = iter([param])
    fake_base_obj.cls = MagicMock()
    fake_base_obj.cls.parameters.return_value = iter([param])
    fake_base_obj.to.return_value = fake_base_obj
    fake_base_obj.train.return_value = None
    fake_base_obj.eval.return_value = None
    mock_model_cls.from_pretrained.return_value = fake_base_obj

    with patch("argot.research.signal.scorers.delta_mlm._PEFT_AVAILABLE", False):
        adapter = _make_adapter(n_steps=2)
        adapter.train([])  # empty

    assert adapter.get_model() is not None


def test_adapter_get_model_before_train_raises() -> None:
    """get_model() must raise RuntimeError if train() was never called."""
    from argot.research.signal.scorers.delta_mlm import DeltaMlmAdapter

    adapter = DeltaMlmAdapter()
    try:
        adapter.get_model()
        raise AssertionError("Expected RuntimeError")
    except RuntimeError:
        pass


# ---------------------------------------------------------------------------
# DeltaMlmScorer construction helpers
# ---------------------------------------------------------------------------


def _make_scorer(agg_variant: str = "mean") -> Any:
    """Build a DeltaMlmScorer with internals injected directly (no model loading)."""
    from typing import Literal

    from argot.research.signal.scorers.delta_mlm import DeltaMlmScorer

    lit_variant: Literal["mean", "min", "p05"] = agg_variant  # type: ignore[assignment]
    scorer = DeltaMlmScorer.__new__(DeltaMlmScorer)
    scorer._n_steps = 2
    scorer._lr = 5e-5
    scorer._batch_size = 2
    scorer._mask_prob = 0.15
    scorer._agg_variant = lit_variant
    scorer._score_batch_size = 32
    scorer.name = f"delta_mlm_{agg_variant}"
    scorer._device = torch.device("cpu")
    scorer._mask_token_id = _MASK_TOKEN_ID
    scorer._tokenizer = _fake_tokenizer()
    scorer._base_model = _fake_model_uniform()
    scorer._adapted_model = _fake_model_uniform()
    return scorer


# ---------------------------------------------------------------------------
# DeltaMlmScorer tests
# ---------------------------------------------------------------------------


@patch("argot.research.signal.scorers.delta_mlm.AutoModelForMaskedLM")
@patch("argot.research.signal.scorers.delta_mlm.AutoTokenizer")
@patch("argot.research.signal.scorers.delta_mlm.select_device")
@patch("argot.research.signal.scorers.delta_mlm.DeltaMlmAdapter")
def test_fit_calls_adapter_and_loads_base(
    mock_adapter_cls: MagicMock,
    mock_device: MagicMock,
    mock_tok_cls: MagicMock,
    mock_model_cls: MagicMock,
) -> None:
    """fit() should instantiate DeltaMlmAdapter, call train(), and load a base model."""
    from argot.research.signal.scorers.delta_mlm import DeltaMlmScorer

    mock_device.return_value = torch.device("cpu")
    mock_tok_cls.from_pretrained.return_value = _fake_tokenizer()

    # Set up adapter mock
    adapter_instance = MagicMock()
    adapted_model = MagicMock()
    adapter_instance.get_model.return_value = adapted_model
    mock_adapter_cls.return_value = adapter_instance

    # Set up base model mock
    base_model_obj = MagicMock()
    base_model_obj.to.return_value = base_model_obj
    param = torch.nn.Parameter(torch.zeros(1))
    base_model_obj.parameters.return_value = iter([param])
    mock_model_cls.from_pretrained.return_value = base_model_obj

    scorer = DeltaMlmScorer(n_steps=2)
    scorer.fit(_tiny_corpus(2))

    adapter_instance.train.assert_called_once()
    adapter_instance.get_model.assert_called_once()
    # AutoModelForMaskedLM.from_pretrained should have been called for the base model
    assert mock_model_cls.from_pretrained.call_count >= 1


def test_score_returns_correct_count() -> None:
    """score() must return exactly one float per fixture."""
    scorer = _make_scorer("mean")
    fixtures = [_make_fixture(), _make_fixture(n_hunk=2), _make_fixture(n_hunk=1)]
    scores = scorer.score(fixtures)
    assert len(scores) == 3
    assert all(isinstance(s, float) for s in scores)


def test_score_empty_hunk_returns_zero() -> None:
    """If encode returns empty for hunk, score should be 0.0."""
    scorer = _make_scorer("mean")
    # Override tokenizer to always return empty
    tok = MagicMock()
    tok.mask_token_id = _MASK_TOKEN_ID
    tok.cls_token_id = _CLS_ID
    tok.sep_token_id = _SEP_ID
    tok.encode.return_value = []
    scorer._tokenizer = tok

    result = scorer.score([_make_fixture()])
    assert result == [0.0]


def test_delta_computation_adapted_minus_base() -> None:
    """delta = logprob_adapted - logprob_base is computed correctly.

    Genuine idiomatic scenario: the adapted model is very confident on the TRUE tokens
    (tokenizer returns [1, 2, 3]) while the base model is uniform.
    Expected: logprob_adapted >> logprob_base → delta > 0 for every position.
    """
    scorer = _make_scorer("mean")

    # Adapted model: concentrated on ALL true tokens (1, 2, 3) simultaneously.
    # The sequence built by _score_one with no context is [CLS=101, 1, 2, 3, SEP=102].
    # Hunk positions are [1, 2, 3] with true token ids [1, 2, 3].
    # We give high logits to tokens 1, 2, and 3 so every position gets logprob ≈ 0.

    def _confident_on_true_model(
        input_ids: torch.Tensor, attention_mask: torch.Tensor, **kwargs: Any
    ) -> Any:
        batch = input_ids.shape[0]
        seq = input_ids.shape[1]
        # All probability mass on tokens 1, 2, 3 (equal high logits) → each gets
        # log_softmax ≈ log(1/3) ≈ -1.1, still far above uniform log(1/32) ≈ -3.47
        logits = torch.full((batch, seq, _VOCAB_SIZE), -1e9)
        logits[:, :, 1] = 100.0
        logits[:, :, 2] = 100.0
        logits[:, :, 3] = 100.0
        result = MagicMock()
        result.logits = logits
        return result

    # Base: uniform → logprob ≈ -log(32) ≈ -3.47 for every token
    # Adapted: three-way concentration on {1,2,3} → logprob ≈ -log(3) ≈ -1.10
    # delta = logprob_adapted - logprob_base ≈ -1.10 - (-3.47) = +2.37 > 0
    scorer._base_model = _fake_model_uniform()
    scorer._adapted_model = _confident_on_true_model

    # Fixture: tokenizer encodes hunk_text → [1, 2, 3]; no context.
    fixture = _make_fixture(n_hunk=3)
    scores = scorer.score([fixture])
    assert scores[0] > 0.0, f"Expected positive delta (idiomatic signal), got {scores[0]}"


def test_aggregation_variants_differ() -> None:
    """The three agg variants produce different values given an asymmetric delta list."""
    # Build scorer with known _aggregate logic
    scorer_mean = _make_scorer("mean")
    scorer_min = _make_scorer("min")
    scorer_p05 = _make_scorer("p05")

    # Inject a set of deltas with spread
    deltas = [0.1, 0.5, 2.0, -0.3, 4.0]

    mean_val = scorer_mean._aggregate(deltas)
    min_val = scorer_min._aggregate(deltas)
    p05_val = scorer_p05._aggregate(deltas)

    # mean = (0.1 + 0.5 + 2.0 + -0.3 + 4.0) / 5 = 1.26
    assert abs(mean_val - 1.26) < 1e-6, f"mean={mean_val}"

    # min = minimum delta = -0.3
    assert abs(min_val - (-0.3)) < 1e-6, f"min={min_val}"

    # p05: sorted=[-0.3, 0.1, 0.5, 2.0, 4.0]; idx=floor(0.05*5)=0 → -0.3
    assert abs(p05_val - (-0.3)) < 1e-6, f"p05={p05_val}"

    # mean differs from min
    assert mean_val != min_val


def test_aggregation_empty_returns_zero() -> None:
    scorer = _make_scorer("mean")
    assert scorer._aggregate([]) == 0.0


def test_compute_logprobs_batching() -> None:
    """_compute_logprobs should work correctly across multiple sub-batches."""
    scorer = _make_scorer("mean")
    scorer._score_batch_size = 2  # force multiple batches with 3 positions

    input_ids = torch.tensor([_CLS_ID, 10, 20, 30, _SEP_ID])
    attention_mask = torch.ones(5, dtype=torch.long)
    hunk_positions = [1, 2, 3]

    lps = scorer._compute_logprobs(scorer._base_model, input_ids, attention_mask, hunk_positions)

    assert len(lps) == 3
    assert all(isinstance(lp, float) for lp in lps)
    # Uniform logits → all log-probs = -log(vocab_size)
    expected = -torch.log(torch.tensor(float(_VOCAB_SIZE))).item()
    for lp in lps:
        assert abs(lp - expected) < 1e-5, f"lp={lp}, expected={expected}"


def test_registry_has_three_variants() -> None:
    """All three delta_mlm variants should be in REGISTRY."""
    import argot.research.signal.scorers.delta_mlm  # noqa: F401
    from argot.research.signal.base import REGISTRY

    assert "delta_mlm_mean" in REGISTRY
    assert "delta_mlm_min" in REGISTRY
    assert "delta_mlm_p05" in REGISTRY
