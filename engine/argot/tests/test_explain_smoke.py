from __future__ import annotations

import json
import sys
from pathlib import Path

import joblib  # type: ignore[import-untyped]
import pygit2
import pytest

import argot.explain as explain_mod
from argot.train import train_model


def _make_records(n: int = 30) -> list[dict]:  # type: ignore[type-arg]
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


def _make_repo(tmp_path: Path, content: str = "x = 1\n") -> pygit2.Repository:
    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    (tmp_path / "main.py").write_text(content)
    repo.index.add("main.py")
    repo.index.write()
    tree = repo.index.write_tree()
    repo.create_commit("refs/heads/main", sig, sig, "init", tree, [])
    repo.set_head("refs/heads/main")
    return repo


def _setup(tmp_path: Path) -> tuple[Path, Path]:
    """Returns (model_path, dataset_path)."""
    bundle = train_model(_make_records(), epochs=1, batch_size=16)
    model_path = tmp_path / "model.pkl"
    joblib.dump(
        {
            "vectorizer": bundle.vectorizer,
            "encoder_state": bundle.model.encoder.state_dict(),
            "predictor_state": bundle.model.predictor.state_dict(),
            "embed_dim": bundle.embed_dim,
            "input_dim": bundle.input_dim,
        },
        model_path,
    )
    dataset_path = tmp_path / "dataset.jsonl"
    dataset_path.write_text("\n".join(json.dumps(r) for r in _make_records(20)))
    return model_path, dataset_path


def test_explain_workdir_mode_clean_repo(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """ref omitted → workdir mode; clean repo produces no output and returns normally."""
    _make_repo(tmp_path)
    model_path, dataset_path = _setup(tmp_path)

    monkeypatch.setattr(
        sys,
        "argv",
        [
            "argot-explain",
            str(tmp_path),
            "--model",
            str(model_path),
            "--dataset",
            str(dataset_path),
        ],
    )
    # Must not raise — clean workdir emits nothing and exits 0
    explain_mod.main()


def test_explain_workdir_mode_with_changes_emits_json(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """ref omitted → workdir mode; changed file scores are emitted as JSON lines."""
    _make_repo(tmp_path)
    model_path, dataset_path = _setup(tmp_path)

    (tmp_path / "main.py").write_text("x = 1\ny = 2\nz = 3\n")

    monkeypatch.setattr(
        sys,
        "argv",
        [
            "argot-explain",
            str(tmp_path),
            "--model",
            str(model_path),
            "--dataset",
            str(dataset_path),
            "--threshold",
            "0.0",  # emit everything
        ],
    )
    capsys.readouterr()  # discard output from _setup (training progress)
    explain_mod.main()

    out = capsys.readouterr().out
    json_lines = [json.loads(ln) for ln in out.splitlines() if ln.strip().startswith("{")]
    assert len(json_lines) > 0
    record = json_lines[0]
    assert record["commit"] == "workdir"
    assert "file_path" in record
    assert "hunk_text" in record
    assert "tag" in record
    assert record["tag"] in {"unusual", "suspicious", "foreign"}
