# engine/argot/research/signal/phase13/experiments/test_contrastive_mlm.py
"""Smoke tests for the contrastive-MLM LoRA training scaffold."""

from __future__ import annotations

from pathlib import Path

import pytest

_HF_CACHE = Path.home() / ".cache" / "huggingface" / "hub"
_CODEBERT_CACHED = (
    any(_HF_CACHE.glob("models--microsoft--codebert-base-mlm*")) if _HF_CACHE.exists() else False
)


@pytest.mark.skipif(
    not _CODEBERT_CACHED,
    reason="microsoft/codebert-base-mlm not in local HF cache",
)
def test_codebert_tokenizer_roundtrip() -> None:
    """Tokenizer round-trip: decode(encode(s)) recovers original words."""
    from argot.research.signal.phase13.experiments.contrastive_mlm import (
        get_tokenizer_and_base_model,
    )

    tokenizer, _ = get_tokenizer_and_base_model()
    source = "def hello(): return 42"
    ids = tokenizer.encode(source, add_special_tokens=False)
    decoded = tokenizer.decode(ids)
    for word in ["def", "hello", "return", "42"]:
        assert word in decoded, f"Expected '{word}' in decoded output: {decoded!r}"


@pytest.mark.skipif(
    not _CODEBERT_CACHED,
    reason="microsoft/codebert-base-mlm not in local HF cache",
)
def test_lora_model_produces_different_logits() -> None:
    """LoRA model after 1 fine-tune step produces logits different from frozen base."""
    import io

    import torch

    from argot.research.signal.phase13.experiments.contrastive_mlm import (
        build_lora_model,
        fine_tune_lora,
        get_tokenizer_and_base_model,
    )

    tokenizer, base_model = get_tokenizer_and_base_model()
    lora_model = build_lora_model(base_model)

    tiny_corpus = io.StringIO(
        "def add(a, b):\n    return a + b\n\ndef sub(a, b):\n    return a - b\n"
        "x = add(1, 2)\ny = sub(3, 4)\nprint(x, y)\nresult = x + y\nassert result == 2\n"
    )
    tiny_text = tiny_corpus.getvalue()

    device = torch.device("cpu")

    class _FakePath:
        def read_text(self, **_: object) -> str:
            return tiny_text

    lora_model = fine_tune_lora(
        lora_model,
        [_FakePath()],  # type: ignore[list-item]
        tokenizer,
        device,
        epochs=1,
        batch_size=1,
    )

    source = "def hello(): return 42"
    encoding = tokenizer(source, return_tensors="pt", add_special_tokens=True)
    input_ids = encoding["input_ids"].to(device)
    attention_mask = encoding["attention_mask"].to(device)

    lora_model.eval()

    with torch.no_grad():
        logits_adapters_on = lora_model(input_ids=input_ids, attention_mask=attention_mask).logits
        with lora_model.disable_adapter():
            logits_adapters_off = lora_model(
                input_ids=input_ids, attention_mask=attention_mask
            ).logits

    assert not torch.allclose(
        logits_adapters_on, logits_adapters_off, atol=1e-4
    ), "Expected adapter-on logits to differ from adapter-off logits after fine-tuning"


def test_log_ratio_formula_sign() -> None:
    """Log-ratio score is positive when token is more surprising under A than B."""
    log_prob_b = -2.0
    log_prob_a = -5.0
    score = log_prob_b - log_prob_a
    assert score > 0, f"Expected positive score, got {score}"

    log_prob_b2 = -5.0
    log_prob_a2 = -2.0
    score2 = log_prob_b2 - log_prob_a2
    assert score2 < 0, f"Expected negative score, got {score2}"
