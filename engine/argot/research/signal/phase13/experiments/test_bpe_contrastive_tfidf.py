"""Tests for phase13 BPE contrastive_tfidf experiment."""

from __future__ import annotations

from pathlib import Path

import pytest

from argot.acceptance.runner import fixture_to_record, load_manifest

_HF_CACHE = Path.home() / ".cache" / "huggingface" / "hub"
_MODEL_CACHED = (
    any(_HF_CACHE.glob("models--microsoft--unixcoder*")) if _HF_CACHE.exists() else False
)

_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
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


def test_bpe_end_to_end_fastapi_no_crash() -> None:
    """BPE FastAPI runner: loads 31 breaks + 20 controls, scores all, returns floats."""
    from argot.research.signal.phase13.experiments.bpe_contrastive_tfidf import (
        _build_model_a_bpe,
        _get_tokenizer,
        _load_model_b_bpe,
        score_records_bpe,
    )

    specs = load_manifest(_FASTAPI_DIR)
    records = [fixture_to_record(_FASTAPI_DIR, spec) for spec in specs]
    assert sum(s.is_break for s in specs) == 31
    assert sum(not s.is_break for s in specs) == 20
    tokenizer = _get_tokenizer()
    model_a, total_a = _build_model_a_bpe(_FASTAPI_DIR, tokenizer)
    model_b, total_b = _load_model_b_bpe()
    scores = score_records_bpe(records, tokenizer, model_a, total_a, model_b, total_b)
    assert len(scores) == 51
    assert all(isinstance(s, float) for s in scores)
