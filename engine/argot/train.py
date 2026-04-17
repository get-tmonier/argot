from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import joblib  # type: ignore[import-untyped]
import torch
from sklearn.feature_extraction.text import TfidfVectorizer  # type: ignore[import-untyped]
from torch.optim import AdamW
from torch.utils.data import DataLoader, TensorDataset

from argot.jepa.encoder import TokenEncoder
from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor

INPUT_DIM = 5000
EMBED_DIM = 192


@dataclass
class ModelBundle:
    vectorizer: TfidfVectorizer
    model: JEPAArgot
    input_dim: int
    embed_dim: int


def train_model(
    records: list[dict[str, Any]],
    *,
    epochs: int = 50,
    batch_size: int = 128,
    lr: float = 5e-5,
    lambd: float = 0.09,
) -> ModelBundle:
    ctx_texts = [" ".join(t["text"] for t in r["context_before"]) for r in records]
    hunk_texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in records]

    vectorizer: TfidfVectorizer = TfidfVectorizer(max_features=INPUT_DIM)
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
    )


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
        },
        out_path,
    )
    print(f"model saved to {out_path}")


if __name__ == "__main__":
    main()
