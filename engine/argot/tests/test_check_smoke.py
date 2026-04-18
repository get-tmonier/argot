from __future__ import annotations

import sys
from pathlib import Path

import joblib  # type: ignore[import-untyped]
import pygit2
import pytest

import argot.check as check_mod
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


def _save_model(tmp_path: Path) -> Path:
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
    return model_path


def test_check_clean_workdir_returns_normally(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """No workdir changes → no results → main() exits 0 (clean)."""
    _make_repo(tmp_path)
    model_path = _save_model(tmp_path)

    monkeypatch.setattr(
        sys,
        "argv",
        ["argot-check", str(tmp_path), "", "--model", str(model_path), "--threshold", "0.5"],
    )
    with pytest.raises(SystemExit) as exc:
        check_mod.main()
    assert exc.value.code == 0


def test_check_workdir_violations_exit_1(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Workdir changes scored above threshold → sys.exit(1)."""
    _make_repo(tmp_path)
    model_path = _save_model(tmp_path)

    (tmp_path / "main.py").write_text("x = 1\ny = 2\nz = 3\n")

    # threshold=-1.0 ensures every scored hunk counts as a violation
    monkeypatch.setattr(
        sys,
        "argv",
        ["argot-check", str(tmp_path), "", "--model", str(model_path), "--threshold", "-1.0"],
    )
    with pytest.raises(SystemExit) as exc:
        check_mod.main()
    assert exc.value.code == 1
