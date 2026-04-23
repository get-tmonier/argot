from __future__ import annotations

import argparse
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import torch
from sklearn.feature_extraction.text import TfidfVectorizer  # type: ignore[import-untyped]
from torch.optim import AdamW
from torch.utils.data import DataLoader, TensorDataset

from argot.jepa.encoder import TokenEncoder
from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor

INPUT_DIM = 5000
EMBED_DIM = 192

_SOURCE_EXTENSIONS: frozenset[str] = frozenset({".py", ".ts", ".tsx"})
_EXCLUDE_DIRS: frozenset[str] = frozenset(
    {
        "node_modules",
        ".git",
        ".tox",
        ".eggs",
        "__pycache__",
        "build",
        "dist",
        ".venv",
        "venv",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        "test",
        "tests",
        "__tests__",
    }
)


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


def _collect_source_files(repo_path: Path) -> list[Path]:
    """Return all source files from repo_path, excluding test and build dirs."""
    files: list[Path] = []
    for p in repo_path.rglob("*"):
        if not p.is_file():
            continue
        if p.suffix not in _SOURCE_EXTENSIONS:
            continue
        parts = set(p.relative_to(repo_path).parts[:-1])
        if parts & _EXCLUDE_DIRS:
            continue
        name = p.name
        if name.startswith("test_") or ".test." in name or ".spec." in name:
            continue
        files.append(p)
    return files


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Collect model-A source files and BPE reference for argot scoring"
    )
    parser.add_argument("--repo", default=".", help="Path to the target repository")
    parser.add_argument(
        "--model-a-out",
        default=".argot/model_a.txt",
        help="Output file listing model-A source paths",
    )
    parser.add_argument(
        "--model-b-out",
        default=".argot/model_b.json",
        help="Output path for the BPE reference JSON",
    )
    args = parser.parse_args()

    repo_path = Path(args.repo).resolve()
    if not (repo_path / ".git").exists():
        print(f"error: not a git repository: {repo_path}", file=sys.stderr)
        sys.exit(2)

    model_a_out = Path(args.model_a_out)
    model_b_out = Path(args.model_b_out)
    model_a_out.parent.mkdir(parents=True, exist_ok=True)
    model_b_out.parent.mkdir(parents=True, exist_ok=True)

    files = _collect_source_files(repo_path)
    if not files:
        print("error: no source files found in repository", file=sys.stderr)
        sys.exit(2)

    model_a_out.write_text("\n".join(str(p) for p in files))
    print(f"model_a: {len(files)} source files → {model_a_out}")

    bpe_ref = Path(__file__).parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
    shutil.copy(bpe_ref, model_b_out)
    print(f"model_b: {model_b_out}")


if __name__ == "__main__":
    main()
