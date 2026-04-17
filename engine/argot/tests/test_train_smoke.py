from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


def _make_dataset(path: Path, n: int = 20) -> None:
    records = [
        {
            "context_before": [
                {"text": "def", "node_type": "keyword", "start_line": 0, "end_line": 0}
            ],
            "hunk_tokens": [
                {"text": f"x{i}", "node_type": "identifier", "start_line": i, "end_line": i}
            ],
            "context_after": [],
        }
        for i in range(n)
    ]
    path.write_text("\n".join(json.dumps(r) for r in records))


def test_train_produces_model(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    dataset = tmp_path / "dataset.jsonl"
    model_out = tmp_path / "model.pkl"
    _make_dataset(dataset)

    import argot.train as train_mod

    monkeypatch.setattr(
        sys,
        "argv",
        ["argot-train", "--dataset", str(dataset), "--out", str(model_out), "--epochs", "2"],
    )
    train_mod.main()

    assert model_out.exists()
    import joblib  # type: ignore[import-untyped]

    bundle = joblib.load(model_out)
    assert "vectorizer" in bundle
    assert "encoder_state" in bundle
    assert "predictor_state" in bundle
