from __future__ import annotations

import torch

from argot.jepa.seq_encoder import MeanPoolEncoder
from argot.jepa.vocab import UNK_ID, Vocab
from argot.train import train_model
from argot.validate import score_records


def _make_records(n: int) -> list[dict]:  # type: ignore[type-arg]
    return [
        {
            "context_before": [{"text": f"tok{i % 10}"}],
            "hunk_tokens": [{"text": f"hunk{i % 5}"}],
            "author_date_iso": str(1000 + i),
            "_repo": "alpha" if i % 2 == 0 else "beta",
        }
        for i in range(n)
    ]


def test_vocab_build_and_encode() -> None:
    records = _make_records(20)
    vocab = Vocab.build(records, max_size=100, min_count=1)
    assert len(vocab.id_to_token) >= 3  # at least specials
    ids = vocab.encode(["tok0", "unknown_token"])
    assert ids[0] != UNK_ID  # tok0 should be in vocab
    assert ids[1] == UNK_ID


def test_mean_pool_encoder_shape() -> None:
    enc = MeanPoolEncoder(vocab_size=100, embed_dim=16, output_dim=32)
    x = torch.zeros(4, 10, dtype=torch.long)
    x[0, 0] = 5  # one real token
    out = enc(x)
    assert out.shape == (4, 32)


def test_token_embed_encoder_kind_and_scores() -> None:
    records = _make_records(60)
    bundle = train_model(records, epochs=1, encoder="token_embed")
    assert bundle.encoder_kind == "token_embed"
    scores = score_records(bundle, records[:5])
    assert len(scores) == 5
    assert all(isinstance(s, float) for s in scores)
