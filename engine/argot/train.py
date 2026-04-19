from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal

import joblib  # type: ignore[import-untyped]
import numpy as np
import torch
from sklearn.feature_extraction.text import TfidfVectorizer  # type: ignore[import-untyped]
from torch.optim import AdamW
from torch.utils.data import DataLoader, TensorDataset

from argot.jepa.bpe_vocab import BpeVocab
from argot.jepa.density_heads import DensityHead, DensityHeadKind, make_head
from argot.jepa.encoder import TokenEncoder
from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor
from argot.jepa.seq_encoder import MeanPoolEncoder
from argot.jepa.vocab import Vocab

INPUT_DIM = 5000
EMBED_DIM = 192

EncoderKind = Literal["tfidf", "word_ngrams", "token_embed", "bpe", "transformer"]


@dataclass
class ModelBundle:
    vectorizer: TfidfVectorizer
    model: JEPAArgot
    input_dim: int
    embed_dim: int
    encoder_kind: EncoderKind = field(default="tfidf")


@dataclass
class DensityBundle:
    bpe_vocab: BpeVocab
    encoder: MeanPoolEncoder
    head: DensityHead
    head_kind: DensityHeadKind


def _train_sklearn_vec(
    records: list[dict[str, Any]],
    *,
    vectorizer: TfidfVectorizer,
    encoder_kind: EncoderKind,
    epochs: int,
    batch_size: int,
    lr: float,
    lambd: float,
) -> ModelBundle:
    def _ctx(r: dict[str, Any]) -> str:
        before = " ".join(t["text"] for t in r["context_before"])
        after = " ".join(t["text"] for t in r.get("context_after", []))
        return f"{before} {after}".strip()

    ctx_texts = [_ctx(r) for r in records]
    hunk_texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in records]

    vectorizer.fit(ctx_texts + hunk_texts)
    actual_input_dim = len(vectorizer.vocabulary_)

    ctx_x = torch.tensor(vectorizer.transform(ctx_texts).toarray(), dtype=torch.float32)
    hunk_x = torch.tensor(vectorizer.transform(hunk_texts).toarray(), dtype=torch.float32)

    loader = DataLoader(
        TensorDataset(ctx_x, hunk_x),
        batch_size=batch_size,
        shuffle=True,
    )

    encoder = TokenEncoder(actual_input_dim, EMBED_DIM)
    predictor = ArgotPredictor(embed_dim=EMBED_DIM)
    model = JEPAArgot(encoder, predictor, lambd=lambd)
    optimizer = AdamW(model.parameters(), lr=lr, weight_decay=1e-3)

    model.train()
    for epoch in range(1, epochs + 1):
        total_loss = 0.0
        for ctx_batch, hunk_batch in loader:
            optimizer.zero_grad()
            losses = model(ctx_batch, hunk_batch)
            losses["loss"].backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            total_loss += losses["loss"].item()
        print(f"epoch {epoch}/{epochs}  loss={total_loss / len(loader):.4f}")

    return ModelBundle(
        vectorizer=vectorizer,
        model=model,
        input_dim=actual_input_dim,
        embed_dim=EMBED_DIM,
        encoder_kind=encoder_kind,
    )


def _train_tfidf(
    records: list[dict[str, Any]],
    *,
    epochs: int,
    batch_size: int,
    lr: float,
    lambd: float,
) -> ModelBundle:
    vectorizer: TfidfVectorizer = TfidfVectorizer(
        max_features=INPUT_DIM,
        analyzer="char_wb",
        ngram_range=(3, 5),
    )
    return _train_sklearn_vec(
        records,
        vectorizer=vectorizer,
        encoder_kind="tfidf",
        epochs=epochs,
        batch_size=batch_size,
        lr=lr,
        lambd=lambd,
    )


def _train_word_ngrams(
    records: list[dict[str, Any]],
    *,
    epochs: int,
    batch_size: int,
    lr: float,
    lambd: float,
) -> ModelBundle:
    vectorizer: TfidfVectorizer = TfidfVectorizer(
        max_features=INPUT_DIM,
        analyzer="word",
        ngram_range=(1, 2),
    )
    return _train_sklearn_vec(
        records,
        vectorizer=vectorizer,
        encoder_kind="word_ngrams",
        epochs=epochs,
        batch_size=batch_size,
        lr=lr,
        lambd=lambd,
    )


_SEQ_LEN = 256
_EMBED_DIM_TOKEN = 128


def _encode_records(
    records: list[dict[str, Any]], vocab: Vocab | BpeVocab, seq_len: int
) -> tuple[torch.Tensor, torch.Tensor]:
    """Encode context_before + hunk_tokens as padded (B, seq_len) long tensors."""
    ctx_ids_list: list[list[int]] = []
    hunk_ids_list: list[list[int]] = []
    for r in records:
        ctx_ids = vocab.encode([t["text"] for t in r["context_before"]])[:seq_len]
        hunk_ids = vocab.encode([t["text"] for t in r["hunk_tokens"]])[:seq_len]
        ctx_ids_list.append(ctx_ids)
        hunk_ids_list.append(hunk_ids)

    def pad(seqs: list[list[int]], length: int) -> torch.Tensor:
        out = torch.zeros(len(seqs), length, dtype=torch.long)
        for i, seq in enumerate(seqs):
            out[i, : len(seq)] = torch.tensor(seq, dtype=torch.long)
        return out

    return pad(ctx_ids_list, seq_len), pad(hunk_ids_list, seq_len)


def _train_token_embed(
    records: list[dict[str, Any]],
    *,
    epochs: int,
    batch_size: int,
    lr: float,
    lambd: float,
) -> ModelBundle:
    vocab = Vocab.build(records, max_size=8000, min_count=2)
    vocab_size = len(vocab.id_to_token)

    ctx_x, hunk_x = _encode_records(records, vocab, _SEQ_LEN)

    loader = DataLoader(
        TensorDataset(ctx_x, hunk_x),
        batch_size=batch_size,
        shuffle=True,
    )

    seq_encoder = MeanPoolEncoder(
        vocab_size=vocab_size, embed_dim=_EMBED_DIM_TOKEN, output_dim=EMBED_DIM
    )
    predictor = ArgotPredictor(embed_dim=EMBED_DIM)
    model = JEPAArgot(seq_encoder, predictor, lambd=lambd)  # type: ignore[arg-type]
    optimizer = AdamW(model.parameters(), lr=lr, weight_decay=1e-3)

    model.train()
    for epoch in range(1, epochs + 1):
        total_loss = 0.0
        for ctx_batch, hunk_batch in loader:
            optimizer.zero_grad()
            losses = model(ctx_batch, hunk_batch)
            losses["loss"].backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            total_loss += losses["loss"].item()
        print(f"epoch {epoch}/{epochs}  loss={total_loss / len(loader):.4f}")

    return ModelBundle(
        vectorizer=vocab,  # Vocab stored in TfidfVectorizer slot (sklearn untyped, mypy accepts)
        model=model,
        input_dim=vocab_size,
        embed_dim=EMBED_DIM,
        encoder_kind="token_embed",
    )


def _train_bpe(
    records: list[dict[str, Any]],
    *,
    epochs: int,
    batch_size: int,
    lr: float,
    lambd: float,
) -> ModelBundle:
    bpe_vocab = BpeVocab.build(records, vocab_size=8000)
    vocab_size = bpe_vocab.vocab_size

    ctx_x, hunk_x = _encode_records(records, bpe_vocab, _SEQ_LEN)

    loader = DataLoader(
        TensorDataset(ctx_x, hunk_x),
        batch_size=batch_size,
        shuffle=True,
    )

    seq_encoder = MeanPoolEncoder(
        vocab_size=vocab_size, embed_dim=_EMBED_DIM_TOKEN, output_dim=EMBED_DIM
    )
    predictor = ArgotPredictor(embed_dim=EMBED_DIM)
    model = JEPAArgot(seq_encoder, predictor, lambd=lambd)  # type: ignore[arg-type]
    optimizer = AdamW(model.parameters(), lr=lr, weight_decay=1e-3)

    model.train()
    for epoch in range(1, epochs + 1):
        total_loss = 0.0
        for ctx_batch, hunk_batch in loader:
            optimizer.zero_grad()
            losses = model(ctx_batch, hunk_batch)
            losses["loss"].backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            total_loss += losses["loss"].item()
        print(f"epoch {epoch}/{epochs}  loss={total_loss / len(loader):.4f}")

    return ModelBundle(
        vectorizer=bpe_vocab,  # BpeVocab stored in TfidfVectorizer slot (sklearn untyped)
        model=model,
        input_dim=vocab_size,
        embed_dim=EMBED_DIM,
        encoder_kind="bpe",
    )


def _get_bpe_embeddings(bundle: ModelBundle, records: list[dict[str, Any]]) -> np.ndarray:
    """Extract mean-pooled BPE embeddings for records using a trained ModelBundle."""
    bpe_vocab = bundle.vectorizer
    if not isinstance(bpe_vocab, BpeVocab):
        raise TypeError(f"expected BpeVocab, got {type(bpe_vocab)}")
    ctx_x, hunk_x = _encode_records(records, bpe_vocab, _SEQ_LEN)
    # Encode hunks only — the density head models the hunk distribution
    bundle.model.encoder.eval()
    with torch.no_grad():
        emb = bundle.model.encoder(hunk_x)
    return emb.numpy()  # type: ignore[no-any-return]


def train_bpe_density(
    records: list[dict[str, Any]],
    *,
    epochs: int,
    batch_size: int,
    lr: float,
    lambd: float,
    head_kind: DensityHeadKind,
    seed: int = 0,
) -> DensityBundle:
    """Train BPE encoder then fit a density head on the resulting embeddings."""
    bundle = _train_bpe(records, epochs=epochs, batch_size=batch_size, lr=lr, lambd=lambd)
    embeddings = _get_bpe_embeddings(bundle, records)
    head = make_head(head_kind, seed=seed)
    head.fit(embeddings)
    bpe_vocab = bundle.vectorizer
    if not isinstance(bpe_vocab, BpeVocab):
        raise TypeError(f"expected BpeVocab, got {type(bpe_vocab)}")
    return DensityBundle(
        bpe_vocab=bpe_vocab,
        encoder=bundle.model.encoder,  # type: ignore[arg-type]
        head=head,
        head_kind=head_kind,
    )


def _train_transformer(
    records: list[dict[str, Any]],
    *,
    epochs: int,
    batch_size: int,
    lr: float,
    lambd: float,
) -> ModelBundle:
    raise NotImplementedError("transformer")


def train_model(
    records: list[dict[str, Any]],
    *,
    epochs: int = 50,
    batch_size: int = 128,
    lr: float = 5e-5,
    lambd: float = 0.09,
    encoder: EncoderKind = "tfidf",
) -> ModelBundle:
    if encoder == "tfidf":
        return _train_tfidf(records, epochs=epochs, batch_size=batch_size, lr=lr, lambd=lambd)
    elif encoder == "word_ngrams":
        return _train_word_ngrams(records, epochs=epochs, batch_size=batch_size, lr=lr, lambd=lambd)
    elif encoder == "token_embed":
        return _train_token_embed(records, epochs=epochs, batch_size=batch_size, lr=lr, lambd=lambd)
    elif encoder == "bpe":
        return _train_bpe(records, epochs=epochs, batch_size=batch_size, lr=lr, lambd=lambd)
    elif encoder == "transformer":
        return _train_transformer(records, epochs=epochs, batch_size=batch_size, lr=lr, lambd=lambd)
    else:
        raise ValueError(f"unknown encoder: {encoder!r}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Train argot JEPA model")
    parser.add_argument("--dataset", default=".argot/dataset.jsonl")
    parser.add_argument("--out", default=".argot/model.pkl")
    parser.add_argument("--epochs", type=int, default=50)
    parser.add_argument("--batch-size", type=int, default=128)
    parser.add_argument("--lr", type=float, default=5e-5)
    parser.add_argument("--lambd", type=float, default=0.09)
    args = parser.parse_args()

    dataset_path = Path(args.dataset)
    if not dataset_path.exists():
        print(f"error: dataset not found at {dataset_path}", file=sys.stderr)
        sys.exit(2)

    records = [json.loads(line) for line in dataset_path.read_text().splitlines() if line.strip()]
    if not records:
        print("error: dataset is empty", file=sys.stderr)
        sys.exit(2)

    bundle = train_model(
        records, epochs=args.epochs, batch_size=args.batch_size, lr=args.lr, lambd=args.lambd
    )

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    joblib.dump(
        {
            "vectorizer": bundle.vectorizer,
            "encoder_state": bundle.model.encoder.state_dict(),
            "predictor_state": bundle.model.predictor.state_dict(),
            "embed_dim": bundle.embed_dim,
            "input_dim": bundle.input_dim,
            "encoder_kind": bundle.encoder_kind,
        },
        out_path,
    )
    print(f"model saved to {out_path}")


if __name__ == "__main__":
    main()
