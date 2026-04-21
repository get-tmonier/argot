"""Tests for phase13 BPE contrastive_tfidf experiment."""

from __future__ import annotations

from pathlib import Path

import pytest

_HF_CACHE = Path.home() / ".cache" / "huggingface" / "hub"
_MODEL_CACHED = (
    any(_HF_CACHE.glob("models--microsoft--unixcoder*")) if _HF_CACHE.exists() else False
)


@pytest.mark.skipif(not _MODEL_CACHED, reason="microsoft/unixcoder-base not in local HF cache")
def test_bpe_subword_count_exceeds_word_count() -> None:
    """BPE tokeniser splits identifiers into subwords: token count > whitespace-split count."""
    from transformers import AutoTokenizer

    tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")  # type: ignore[no-untyped-call]
    source = "paramtype_convert context_init_tail argparse_class_based"
    word_count = len(source.split())
    bpe_ids = tokenizer.encode(source, add_special_tokens=False)
    assert (
        len(bpe_ids) > word_count
    ), f"Expected BPE count ({len(bpe_ids)}) > word count ({word_count})"
