from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest
import torch
from torch import nn

from argot.jepa import pretrained_encoder as pe_mod
from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device


class _FakeSentenceTransformer(nn.Module):
    """Mimics just enough of `SentenceTransformer` for PretrainedEncoder tests."""

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__()
        self._dim = 16
        self._dummy = nn.Linear(1, 1)

    def get_sentence_embedding_dimension(self) -> int:
        return self._dim

    def encode(
        self,
        texts: list[str],
        *,
        batch_size: int = 32,
        convert_to_tensor: bool = False,
        show_progress_bar: bool = False,
        normalize_embeddings: bool = False,
    ) -> torch.Tensor:
        gen = torch.Generator().manual_seed(0)
        return torch.randn(len(texts), self._dim, generator=gen)


@pytest.fixture(autouse=True)
def _patch_sentence_transformer(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(pe_mod, "SentenceTransformer", _FakeSentenceTransformer)


def test_select_device_returns_torch_device() -> None:
    device = select_device()
    assert isinstance(device, torch.device)
    assert device.type in {"cuda", "mps", "cpu"}


def test_pretrained_encoder_load_and_shape() -> None:
    enc = PretrainedEncoder(model_name="fake-model", device="cpu")
    assert enc.embed_dim == 16

    out = enc.encode_texts(["def foo(): pass", "class Bar: pass"])
    assert out.shape == (2, 16)


def test_pretrained_encoder_empty_input() -> None:
    enc = PretrainedEncoder(model_name="fake-model", device="cpu")
    out = enc.encode_texts([])
    assert out.shape == (0, 16)


def test_pretrained_encoder_empty_string_safe() -> None:
    enc = PretrainedEncoder(model_name="fake-model", device="cpu")
    out = enc.encode_texts(["", "  "])
    assert out.shape == (2, 16)


def test_pretrained_encoder_weights_are_frozen() -> None:
    enc = PretrainedEncoder(model_name="fake-model", device="cpu")
    assert all(not p.requires_grad for p in enc._model.parameters())


def test_forward_is_passthrough() -> None:
    enc = PretrainedEncoder(model_name="fake-model", device="cpu")
    x = torch.randn(3, 16)
    out = enc.forward(x)
    assert torch.equal(out, x)


def _make_records(n: int) -> list[dict[str, Any]]:
    return [
        {
            "context_before": [{"text": f"def foo_{i}"}],
            "hunk_tokens": [{"text": f"return {i}"}],
            "author_date_iso": str(1000 + i),
            "_repo": "alpha" if i % 2 == 0 else "beta",
        }
        for i in range(n)
    ]


def test_train_pretrained_dispatch_produces_bundle() -> None:
    from argot.train import train_model

    records = _make_records(30)
    bundle = train_model(records, epochs=1, batch_size=8, encoder="pretrained")
    assert bundle.encoder_kind == "pretrained"
    assert bundle.embed_dim == 16
    assert isinstance(bundle.vectorizer, PretrainedEncoder)


def test_vectorize_pretrained_routes_through_dispatch() -> None:
    from argot.train import train_model
    from argot.validate import score_records

    records = _make_records(30)
    bundle = train_model(records, epochs=1, batch_size=8, encoder="pretrained")
    scores = score_records(bundle, records[:5])
    assert len(scores) == 5
    assert all(isinstance(s, float) for s in scores)


def test_run_benchmark_with_pretrained_encoder(tmp_path: Path) -> None:
    from argot.corpus import run_benchmark

    dataset = tmp_path / "tagged.jsonl"
    out = tmp_path / "results.jsonl"
    records = []
    base_ts = 1_700_000_000
    for repo in ("home", "foreign"):
        for i in range(20):
            records.append(
                {
                    "_repo": repo,
                    "author_date_iso": str(base_ts + i * 3600),
                    "language": "python",
                    "context_before": [{"text": f"{repo}_ctx_{i}"}],
                    "hunk_tokens": [{"text": f"{repo}_hunk_{i}"}],
                    "context_after": [],
                }
            )
    dataset.write_text("\n".join(json.dumps(r) for r in records) + "\n")

    run_benchmark(
        dataset=dataset,
        sizes=[20],
        seeds=1,
        output=out,
        batch_size=8,
        encoder="pretrained",
        epochs=1,
    )
    rows = [json.loads(line) for line in out.read_text().strip().splitlines()]
    assert len(rows) == 1
    row = rows[0]
    assert row["encoder"] == "pretrained"
    assert "synthetic_auc_mean" in row
