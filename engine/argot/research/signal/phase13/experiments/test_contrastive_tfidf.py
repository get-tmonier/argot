"""Tests for phase13 contrastive_tfidf experiment."""

from __future__ import annotations

from pathlib import Path

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.research.signal.phase13.experiments.contrastive_tfidf import (
    _build_model_a,
    _load_model_b,
    score_records,
)

_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)


def test_smoke_model_a_equals_model_b() -> None:
    """When model_A = model_B, all log-ratio scores must be exactly 0."""
    specs = load_manifest(_FASTAPI_DIR)
    records = [fixture_to_record(_FASTAPI_DIR, spec) for spec in specs]
    model_b, total_b = _load_model_b()
    scores = score_records(records, model_b, total_b, model_b, total_b)
    assert max(abs(s) for s in scores) < 1e-9


def test_end_to_end_fastapi_no_crash() -> None:
    """End-to-end: loads exactly 31 breaks + 20 controls, scores all 51, returns floats."""
    specs = load_manifest(_FASTAPI_DIR)
    records = [fixture_to_record(_FASTAPI_DIR, spec) for spec in specs]
    assert sum(s.is_break for s in specs) == 31
    assert sum(not s.is_break for s in specs) == 20
    model_a, total_a = _build_model_a(_FASTAPI_DIR)
    model_b, total_b = _load_model_b()
    scores = score_records(records, model_a, total_a, model_b, total_b)
    assert len(scores) == 51
    assert all(isinstance(s, float) for s in scores)
