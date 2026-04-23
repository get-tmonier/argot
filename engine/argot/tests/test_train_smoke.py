from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from argot.train import ModelBundle, train_model


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


def _make_records(n: int) -> list[dict]:  # type: ignore[type-arg]
    return [
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


def test_train_model_returns_bundle(tmp_path: Path) -> None:
    records = _make_records(50)
    bundle = train_model(records, epochs=1, batch_size=16)
    assert isinstance(bundle, ModelBundle)
    assert bundle.input_dim > 0
    assert bundle.embed_dim == 192
    assert bundle.vectorizer is not None
    assert bundle.model is not None


def test_train_produces_model(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """argot-train writes model_a.txt and model_b.json into --argot-dir."""
    import pygit2

    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    (tmp_path / "main.py").write_text("x = 1\n")
    repo.index.add("main.py")
    repo.index.write()
    tree = repo.index.write_tree()
    repo.create_commit("refs/heads/main", sig, sig, "init", tree, [])

    argot_dir = tmp_path / ".argot"
    argot_dir.mkdir()

    import argot.train as train_mod

    monkeypatch.setattr(
        sys,
        "argv",
        [
            "argot-train",
            "--repo",
            str(tmp_path),
            "--model-a-out",
            str(argot_dir / "model_a.txt"),
            "--model-b-out",
            str(argot_dir / "model_b.json"),
        ],
    )
    train_mod.main()

    assert (argot_dir / "model_a.txt").exists()
    assert (argot_dir / "model_b.json").exists()
    lines = (argot_dir / "model_a.txt").read_text().splitlines()
    assert any("main.py" in ln for ln in lines)
