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


def test_bpe_end_to_end_click_no_crash() -> None:
    """BPE click runner: loads 8 breaks + 10 controls, scores all, returns floats."""
    import json
    import os

    from argot.acceptance.runner import FixtureSpec, fixture_to_record
    from argot.research.signal.phase13.experiments.bpe_contrastive_tfidf import _get_tokenizer
    from argot.research.signal.phase13.experiments.bpe_contrastive_tfidf_click import (
        _build_model_a_bpe_click,
        _load_model_b_bpe,
        score_records_bpe,
    )

    fixture_dir = Path(__file__).parent.parent / "tier3_fixtures" / "click"
    manifest_path = fixture_dir / "manifest_matched.json"

    manifest = json.loads(manifest_path.read_text())
    records = []
    is_break_list = []
    for f in manifest["fixtures"]:
        spec = FixtureSpec(
            name=f["name"],
            scope="default",
            file=f["file"],
            hunk_start_line=f["hunk_start_line"],
            hunk_end_line=f["hunk_end_line"],
            is_break=f["is_break"],
            rationale=f.get("rationale", ""),
            category=f.get("category", "control" if not f["is_break"] else "break"),
        )
        records.append(fixture_to_record(fixture_dir, spec, "file_only"))
        is_break_list.append(spec.is_break)
    assert sum(is_break_list) == 8
    assert sum(not b for b in is_break_list) == 10
    click_dir = Path(os.environ.get("CLICK_DIR", "/tmp/click-clone"))
    if not click_dir.is_dir():
        pytest.skip("CLICK_DIR not available; set env var to run this test")
    tokenizer = _get_tokenizer()
    model_a, total_a = _build_model_a_bpe_click(click_dir, tokenizer)
    model_b, total_b = _load_model_b_bpe()
    scores = score_records_bpe(records, tokenizer, model_a, total_a, model_b, total_b)
    assert len(scores) == 18
    assert all(isinstance(s, float) for s in scores)
