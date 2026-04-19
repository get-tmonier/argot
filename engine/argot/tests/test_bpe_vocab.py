from __future__ import annotations

from argot.jepa.bpe_vocab import PAD_ID, UNK_ID, BpeVocab
from argot.train import train_model
from argot.validate import score_records


def _make_records(n: int) -> list[dict]:  # type: ignore[type-arg]
    return [
        {
            "context_before": [{"text": f"token{i % 10}"}],
            "hunk_tokens": [{"text": f"hunk{i % 5}"}],
            "author_date_iso": str(1000 + i),
            "_repo": "alpha" if i % 2 == 0 else "beta",
        }
        for i in range(n)
    ]


def test_bpe_vocab_build_and_encode() -> None:
    records = _make_records(40)
    bpe = BpeVocab.build(records, vocab_size=200)
    assert bpe.vocab_size >= 3  # at least specials
    ids = bpe.encode(["token0", "hunk1"])
    assert isinstance(ids, list)
    assert len(ids) > 0
    assert all(isinstance(i, int) for i in ids)


def test_bpe_special_token_ids() -> None:
    records = _make_records(40)
    bpe = BpeVocab.build(records, vocab_size=200)
    vocab = bpe._tokenizer.get_vocab()
    assert vocab["<pad>"] == PAD_ID
    assert vocab["<unk>"] == UNK_ID


def test_bpe_vocab_state_dict_roundtrip() -> None:
    records = _make_records(40)
    bpe = BpeVocab.build(records, vocab_size=200)
    state = bpe.state_dict()
    bpe2 = BpeVocab.from_state_dict(state)
    assert bpe.encode(["token0"]) == bpe2.encode(["token0"])


def test_bpe_encoder_kind_and_scores() -> None:
    records = _make_records(60)
    bundle = train_model(records, epochs=1, encoder="bpe")
    assert bundle.encoder_kind == "bpe"
    scores = score_records(bundle, records[:5])
    assert len(scores) == 5
    assert all(isinstance(s, float) for s in scores)
