"""Unit tests for PretrainedEncoder HF direct path (mean-pool + normalize).

These tests verify the encoding mechanics using a tiny mock model — no internet,
no large model download. They catch implementation bugs independently of whether
the real UnixCoder embeddings are semantically useful.
"""

from __future__ import annotations

import math
from typing import Any
from unittest.mock import MagicMock, patch

import pytest
import torch
import torch.nn.functional as F  # noqa: N812

from argot.jepa import pretrained_encoder as pe_mod
from argot.jepa.predictor import ArgotPredictor
from argot.jepa.pretrained_encoder import PretrainedEncoder

HF_MODEL = "microsoft/unixcoder-base"
_HIDDEN_DIM = 32
_VOCAB_SIZE = 100


class _FakeTokenizerOutput(dict[str, torch.Tensor]):
    """dict-like tokenizer output with input_ids and attention_mask."""


class _FakeTokenizer:
    """Minimal tokenizer mock: encodes text as token counts, pads to batch max."""

    def __call__(
        self,
        texts: list[str],
        *,
        padding: bool = True,
        truncation: bool = True,
        max_length: int = 512,
        return_tensors: str = "pt",
    ) -> dict[str, torch.Tensor]:
        lengths = [min(len(t.split()) + 2, max_length) for t in texts]  # +2 for CLS/SEP
        max_len = max(lengths)
        input_ids = torch.zeros(len(texts), max_len, dtype=torch.long)
        attention_mask = torch.zeros(len(texts), max_len, dtype=torch.long)
        for i, length in enumerate(lengths):
            input_ids[i, :length] = 1
            attention_mask[i, :length] = 1
        return {"input_ids": input_ids, "attention_mask": attention_mask}


class _FakeHFModelOutput:
    def __init__(self, last_hidden_state: torch.Tensor) -> None:
        self.last_hidden_state = last_hidden_state


class _FakeHFModel(torch.nn.Module):
    """Returns a constant hidden state per token so pooling results are predictable."""

    def __init__(self) -> None:
        super().__init__()
        self.config = MagicMock()
        self.config.hidden_size = _HIDDEN_DIM
        # Each token position has a fixed embedding = position index as float
        self._linear = torch.nn.Linear(1, _HIDDEN_DIM, bias=False)

    def forward(  # noqa: N803
        self, input_ids: torch.Tensor, attention_mask: torch.Tensor, **_: Any
    ) -> _FakeHFModelOutput:
        b, seq_len = input_ids.shape
        # hidden[b, pos, :] = (pos + 1) * ones — predictable for mask tests
        positions = torch.arange(1, seq_len + 1, dtype=torch.float32).unsqueeze(0).unsqueeze(-1)
        hidden = positions.expand(b, seq_len, _HIDDEN_DIM)
        return _FakeHFModelOutput(hidden)

    def parameters(self) -> Any:  # type: ignore[override]
        return iter([self._linear.weight])

    def eval(self) -> _FakeHFModel:
        return self

    def to(self, device: Any) -> _FakeHFModel:  # type: ignore[override]
        return self


@pytest.fixture()
def hf_encoder() -> PretrainedEncoder:
    """PretrainedEncoder with mocked HF tokenizer + model (HF direct path)."""
    fake_tokenizer = _FakeTokenizer()
    fake_model = _FakeHFModel()

    with (
        patch.object(pe_mod, "AutoTokenizer") as mock_tok_cls,
        patch.object(pe_mod, "AutoModel") as mock_model_cls,
    ):
        mock_tok_cls.from_pretrained.return_value = fake_tokenizer
        mock_model_cls.from_pretrained.return_value = fake_model
        enc = PretrainedEncoder(model_name=HF_MODEL, device="cpu")

    return enc


# ---------------------------------------------------------------------------
# Shape and basic properties
# ---------------------------------------------------------------------------


def test_hf_encoder_embed_dim(hf_encoder: PretrainedEncoder) -> None:
    assert hf_encoder.embed_dim == _HIDDEN_DIM
    assert hf_encoder._use_hf_direct is True


def test_hf_encoder_output_shape(hf_encoder: PretrainedEncoder) -> None:
    texts = ["def foo(): pass", "class Bar: pass", "import os"]
    out = hf_encoder.encode_texts(texts)
    assert out.shape == (3, _HIDDEN_DIM)


def test_hf_encoder_empty_input(hf_encoder: PretrainedEncoder) -> None:
    out = hf_encoder.encode_texts([])
    assert out.shape == (0, _HIDDEN_DIM)


# ---------------------------------------------------------------------------
# Mask correctness: padding tokens must be excluded from mean-pool
# ---------------------------------------------------------------------------


def test_hf_encoder_mask_excludes_padding(hf_encoder: PretrainedEncoder) -> None:
    """Mean-pool must exclude padded positions.

    The fake model returns hidden[b, pos, :] = (pos + 1) * ones, so:
    - A 3-token sequence: mean = (1+2+3)/3 = 2.0 in every dim
    - A 5-token sequence: mean = (1+2+3+4+5)/5 = 3.0 in every dim

    If padding is ignored correctly, a short text and long text produce DIFFERENT
    pooled vectors. If padding leaks in, both converge toward 3+ regardless of length.
    """
    short_text = "hi"  # _FakeTokenizer gives ~3 tokens (splits on spaces + 2 special)
    long_text = "a b c d e f"  # _FakeTokenizer gives ~8 tokens

    out = hf_encoder.encode_texts([short_text, long_text], normalize_embeddings=False)
    # Both rows should be the same in every dim (constant hidden per dim by fake model),
    # but the MAGNITUDE of the mean depends on sequence length (longer → higher mean)
    short_mean = out[0, 0].item()
    long_mean = out[1, 0].item()
    # short sequence mean < long sequence mean because positions 1..N average higher for longer N
    assert short_mean < long_mean, (
        f"Padding not excluded: short_mean={short_mean:.4f} should be < long_mean={long_mean:.4f}. "
        "If equal, padding zeros are being averaged in."
    )


# ---------------------------------------------------------------------------
# Normalization
# ---------------------------------------------------------------------------


def test_hf_encoder_normalize_produces_unit_vectors(hf_encoder: PretrainedEncoder) -> None:
    texts = ["def foo(): pass", "import os", "for i in range(10): print(i)"]
    out = hf_encoder.encode_texts(texts, normalize_embeddings=True)
    norms = out.norm(dim=-1)
    assert torch.allclose(
        norms, torch.ones_like(norms), atol=1e-5
    ), f"Expected unit vectors after normalize_embeddings=True, got norms: {norms}"


def test_hf_encoder_no_normalize_not_unit_vectors(hf_encoder: PretrainedEncoder) -> None:
    texts = ["def foo(): pass", "import os"]
    out = hf_encoder.encode_texts(texts, normalize_embeddings=False)
    norms = out.norm(dim=-1)
    # With our fake model, the norms should be > 1 (not unit vectors)
    assert not torch.allclose(
        norms, torch.ones_like(norms), atol=1e-2
    ), "Expected non-unit norms when normalize_embeddings=False"


# ---------------------------------------------------------------------------
# Batching consistency
# ---------------------------------------------------------------------------


def test_hf_encoder_batching_matches_single(hf_encoder: PretrainedEncoder) -> None:
    """Encoding texts one at a time should match encoding them in a batch."""
    texts = ["def foo(): pass", "class Bar: pass", "import os"]
    batch_out = hf_encoder.encode_texts(texts, normalize_embeddings=False)
    for i, text in enumerate(texts):
        single_out = hf_encoder.encode_texts([text], normalize_embeddings=False)
        assert torch.allclose(
            batch_out[i], single_out[0], atol=1e-5
        ), f"Batch vs single mismatch at index {i}"


# ---------------------------------------------------------------------------
# Semantic sanity (requires real model — marks as integration)
# These tests verify that the REAL UnixCoder produces discriminative embeddings.
# They are skipped if the model is not cached locally.
# ---------------------------------------------------------------------------


def _unixcoder_available() -> bool:
    try:
        from transformers import AutoTokenizer

        AutoTokenizer.from_pretrained(  # type: ignore[no-untyped-call]
            "microsoft/unixcoder-base", local_files_only=True
        )
        return True
    except Exception:
        return False


@pytest.mark.skipif(not _unixcoder_available(), reason="UnixCoder not cached locally")
def test_unixcoder_different_snippets_have_different_embeddings() -> None:
    """Two very different code snippets should NOT have cosine similarity = 1.0."""
    enc = PretrainedEncoder(model_name=HF_MODEL, device="cpu")
    texts = [
        "def add(a, b): return a + b",
        "class DatabaseConnection:\n    def connect(self): pass",
    ]
    out = enc.encode_texts(texts, normalize_embeddings=True)
    sim = (out[0] @ out[1]).item()
    assert sim < 0.9999, f"Different snippets should not be identical: cosine_sim={sim:.6f}"


@pytest.mark.skipif(not _unixcoder_available(), reason="UnixCoder not cached locally")
def test_unixcoder_pairwise_similarity_has_variance() -> None:
    """Pairwise cosine similarity across diverse snippets must have std > 0.001.

    If std ≈ 0, all embeddings collapse to the same direction and JEPA MSE
    signal is zero — explaining delta=0.002 in the audit experiment.
    """
    enc = PretrainedEncoder(model_name=HF_MODEL, device="cpu")
    snippets = [
        "def add(a, b): return a + b",
        "class Foo: pass",
        "import numpy as np",
        "for i in range(10): print(i)",
        "async def fetch(url): return await session.get(url)",
    ]
    out = enc.encode_texts(snippets, normalize_embeddings=True)
    sims: list[float] = []
    for i in range(out.shape[0]):
        for j in range(i + 1, out.shape[0]):
            sims.append((out[i] @ out[j]).item())
    import statistics

    std = statistics.stdev(sims)
    mean = statistics.mean(sims)
    assert std > 0.001, (
        f"UnixCoder pairwise cosine sim has near-zero variance: mean={mean:.4f} std={std:.6f}. "
        "Embeddings are collapsed — JEPA cannot discriminate code snippets."
    )


# ---------------------------------------------------------------------------
# JEPA predictor scale compatibility test
# Catches the normalize_embeddings=True bug: ArgotPredictor ends with LayerNorm,
# giving output magnitude ≈ sqrt(embed_dim). Unit-sphere targets have magnitude 1.
# That 27× scale gap makes JEPA MSE insensitive to direction — delta collapses.
# ---------------------------------------------------------------------------


def test_jepa_predictor_scale_matches_unit_targets() -> None:
    """ArgotPredictor output magnitude must be comparable to normalized targets.

    ArgotPredictor ends with LayerNorm, so each output element has std≈1 and
    total magnitude ≈ sqrt(embed_dim). When targets are unit vectors (magnitude=1),
    the scale ratio is sqrt(embed_dim) ≈ 27 for embed_dim=768.

    MSE is then dominated by the scale gap: (27 - 1)² per element ≈ 0.90 averaged
    over 768 dims, regardless of direction. Every chunk scores ≈ 0.90, break_mean ≈
    ctrl_mean, delta ≈ 0. This is the root cause of the UnixCoder + normalize=True
    collapse observed in the static-chunk audit (delta 0.1817 → 0.0022).

    Fix: do NOT normalize embeddings when using JEPA MSE scoring. Let the predictor
    operate in the raw embedding space where targets have comparable magnitude.
    """
    embed_dim = 768
    predictor = ArgotPredictor(embed_dim=embed_dim, depth=6, mlp_dim=1024)
    predictor.eval()

    # Normalized unit-sphere inputs (what the JEPA sees with normalize_embeddings=True)
    unit_inputs = F.normalize(torch.randn(8, embed_dim), dim=-1)  # magnitude = 1.0
    unit_targets = F.normalize(torch.randn(8, embed_dim), dim=-1)  # magnitude = 1.0

    with torch.no_grad():
        pred_output = predictor(unit_inputs.unsqueeze(1)).squeeze(1)

    pred_mag = pred_output.norm(dim=-1).mean().item()
    target_mag = unit_targets.norm(dim=-1).mean().item()  # = 1.0

    expected_pred_mag = math.sqrt(embed_dim)  # ≈ 27.7 from LayerNorm
    scale_ratio = pred_mag / target_mag

    # The predictor output should be close to sqrt(embed_dim) (LayerNorm behaviour)
    assert (
        abs(pred_mag - expected_pred_mag) < expected_pred_mag * 0.5
    ), f"Unexpected predictor magnitude: got {pred_mag:.2f}, expected ≈{expected_pred_mag:.1f}"

    # Core assertion: scale ratio >> 1 means normalize_embeddings=True is incompatible with JEPA
    assert scale_ratio > 5.0, (
        f"Expected scale_ratio > 5 (predictor ≫ unit targets), got {scale_ratio:.2f}. "
        "If this fails, the predictor architecture changed — "
        "re-evaluate normalize_embeddings usage."
    )

    # Derived: MSE between predictor output and unit targets is dominated by scale, not direction
    mse_scale_component = (pred_mag - target_mag) ** 2 / embed_dim
    assert mse_scale_component > 0.8, (
        f"Scale-mismatch MSE component = {mse_scale_component:.4f} < 0.8. "
        "delta collapse may not occur — check if this test is still relevant."
    )
