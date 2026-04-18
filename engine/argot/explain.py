from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import joblib  # type: ignore[import-untyped]
import numpy as np
import pygit2
import torch

from argot.check import _resolve_shas, _workdir_patches
from argot.git_walk import walk_commits
from argot.jepa.encoder import TokenEncoder
from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor
from argot.tokenize import language_for_path, tokenize_lines


def percentile_rank(value: float, distribution: list[float]) -> float:
    arr = np.array(distribution)
    return float(np.mean(arr < value) * 100)


def _score_to_tag(score: float, threshold: float) -> str:
    if score <= threshold + 0.3:
        return "unusual"
    elif score <= threshold + 0.6:
        return "suspicious"
    else:
        return "foreign"


def select_style_examples(records: list[dict[str, Any]], *, n: int = 5) -> list[dict[str, Any]]:
    """Pick n lowest-surprise records, one per file where possible."""
    sorted_records = sorted(records, key=lambda r: r["_score"])
    seen_files: set[str] = set()
    diverse: list[dict[str, Any]] = []
    remainder: list[dict[str, Any]] = []

    for r in sorted_records:
        fp = r.get("file_path", "")
        if fp not in seen_files:
            seen_files.add(fp)
            diverse.append(r)
        else:
            remainder.append(r)
        if len(diverse) >= n:
            break

    result = diverse[:n]
    if len(result) < n:
        result += remainder[: n - len(result)]
    return result


def _score_dataset(
    model: JEPAArgot,
    vectorizer: Any,
    records: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    ctx_texts = [" ".join(t["text"] for t in r["context_before"]) for r in records]
    hunk_texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in records]
    ctx_x = torch.tensor(vectorizer.transform(ctx_texts).toarray(), dtype=torch.float32)
    hunk_x = torch.tensor(vectorizer.transform(hunk_texts).toarray(), dtype=torch.float32)
    model.eval()
    scored = []
    with torch.no_grad():
        for i, record in enumerate(records):
            score = model.surprise(ctx_x[i : i + 1], hunk_x[i : i + 1]).item()
            scored.append({**record, "_score": score})
    return scored


def main() -> None:
    parser = argparse.ArgumentParser(description="Explain style anomalies in a git ref")
    parser.add_argument("repo_path")
    parser.add_argument("ref", nargs="?", default="")
    parser.add_argument("--model", default=".argot/model.pkl")
    parser.add_argument("--dataset", default=".argot/dataset.jsonl")
    parser.add_argument("--threshold", type=float, default=0.5)
    parser.add_argument("--examples", type=int, default=5)
    args = parser.parse_args()

    model_path = Path(args.model)
    if not model_path.exists():
        print(f"error: model not found at {model_path}", file=sys.stderr)
        sys.exit(2)

    dataset_path = Path(args.dataset)
    if not dataset_path.exists():
        print(f"error: dataset not found at {dataset_path}", file=sys.stderr)
        sys.exit(2)

    bundle = joblib.load(model_path)
    vectorizer = bundle["vectorizer"]
    input_dim: int = bundle["input_dim"]
    embed_dim: int = bundle["embed_dim"]

    encoder = TokenEncoder(input_dim, embed_dim)
    encoder.load_state_dict(bundle["encoder_state"])
    predictor = ArgotPredictor(embed_dim=embed_dim)
    predictor.load_state_dict(bundle["predictor_state"])
    model = JEPAArgot(encoder, predictor)
    model.eval()

    raw_dataset = [
        json.loads(line) for line in dataset_path.read_text().splitlines() if line.strip()
    ]
    scored_dataset = _score_dataset(model, vectorizer, raw_dataset)
    distribution = [r["_score"] for r in scored_dataset]
    style_examples = select_style_examples(scored_dataset, n=args.examples)
    example_texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in style_examples]

    context_lines = 50

    def _emit_patches(patches: Any, commit_label: str) -> None:
        with torch.no_grad():
            for file_path, post_blob, hunks in patches:
                lang = language_for_path(file_path)
                if lang is None:
                    continue
                try:
                    source_lines = post_blob.decode("utf-8", errors="replace").splitlines()
                except Exception:
                    continue

                for hunk in hunks:
                    hunk_start = hunk.new_start - 1
                    hunk_end = hunk_start + hunk.new_lines
                    if hunk_start < 0 or hunk_end > len(source_lines):
                        continue

                    before_start = max(0, hunk_start - context_lines)
                    ctx_tokens = tokenize_lines(source_lines, lang, before_start, hunk_start)
                    hunk_tokens = tokenize_lines(source_lines, lang, hunk_start, hunk_end)

                    ctx_text = " ".join(t.text for t in ctx_tokens)
                    hunk_text = " ".join(t.text for t in hunk_tokens)

                    ctx_vec = torch.tensor(
                        vectorizer.transform([ctx_text]).toarray(), dtype=torch.float32
                    )
                    hunk_vec = torch.tensor(
                        vectorizer.transform([hunk_text]).toarray(), dtype=torch.float32
                    )

                    score = model.surprise(ctx_vec, hunk_vec).item()
                    pct = percentile_rank(score, distribution)

                    if score <= args.threshold:
                        continue

                    print(
                        json.dumps(
                            {
                                "file_path": file_path,
                                "line": hunk.new_start,
                                "commit": commit_label,
                                "surprise": round(score, 4),
                                "percentile": round(pct, 1),
                                "tag": _score_to_tag(score, args.threshold),
                                "hunk_text": hunk_text,
                                "context_text": ctx_text,
                                "style_examples": example_texts,
                            }
                        )
                    )

    if args.ref == "":
        _emit_patches(_workdir_patches(args.repo_path), "workdir")
    else:
        repo = pygit2.Repository(args.repo_path)
        shas = _resolve_shas(repo, args.ref)
        if not shas:
            sys.exit(0)
        _emit_patches(
            ((fp, blob, hunks) for _, fp, blob, hunks in walk_commits(args.repo_path, shas)),
            args.ref,
        )


if __name__ == "__main__":
    main()
