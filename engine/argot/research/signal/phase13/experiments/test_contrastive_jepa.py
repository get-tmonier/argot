"""Tests for phase13 contrastive_jepa experiment."""

from __future__ import annotations

from pathlib import Path

import torch

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor
from argot.jepa.pretrained_encoder import PretrainedEncoder
from argot.research.signal.phase13.experiments.contrastive_jepa import (
    _hunk_text,
    score_contrastive,
)
from argot.train import ModelBundle

_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)


def _real_bundle() -> ModelBundle:
    """Tiny real bundle (real encoder, 1-layer predictor) for shape tests."""
    encoder = PretrainedEncoder()
    predictor = ArgotPredictor(embed_dim=encoder.embed_dim, depth=1, mlp_dim=32)
    model = JEPAArgot(torch.nn.Identity(), predictor, lambd=0.09)  # type: ignore[arg-type]
    return ModelBundle(
        vectorizer=encoder,
        model=model,
        input_dim=encoder.embed_dim,
        embed_dim=encoder.embed_dim,
        encoder_kind="pretrained",
    )


def test_hunk_text_returns_string() -> None:
    specs = load_manifest(_FASTAPI_DIR)
    record = fixture_to_record(_FASTAPI_DIR, specs[0])
    text = _hunk_text(record)
    assert isinstance(text, str) and len(text) > 0


def test_score_contrastive_empty() -> None:
    """Empty records → empty scores without calling the encoder."""
    bundle = _real_bundle()
    scores = score_contrastive(bundle, [])
    assert scores == []


def test_score_contrastive_shape() -> None:
    """score_contrastive returns one float per record."""
    specs = load_manifest(_FASTAPI_DIR)[:3]
    records = [fixture_to_record(_FASTAPI_DIR, spec) for spec in specs]
    bundle = _real_bundle()
    scores = score_contrastive(bundle, records)
    assert len(scores) == 3
    assert all(isinstance(s, float) for s in scores)
