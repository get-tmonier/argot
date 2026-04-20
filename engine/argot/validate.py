from __future__ import annotations

import argparse
import json
import random
import sys
from pathlib import Path
from typing import Any, Literal

import numpy as np
import torch
import torch.nn.functional as F  # noqa: N812
from sklearn.metrics import roc_auc_score  # type: ignore[import-untyped]

from argot.jepa.bpe_vocab import BpeVocab
from argot.jepa.pretrained_encoder import PretrainedEncoder
from argot.jepa.seq_encoder import MeanPoolEncoder  # noqa: F401  (used for isinstance check)
from argot.jepa.vocab import Vocab
from argot.train import ModelBundle, train_model

StratifyBy = Literal["none", "language", "top-dir"]


def split_by_time(
    records: list[dict[str, Any]], *, ratio: float = 0.8
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    sorted_records = sorted(records, key=lambda r: int(r["author_date_iso"]))
    split_idx = int(len(sorted_records) * ratio)
    return sorted_records[:split_idx], sorted_records[split_idx:]


def shuffle_negatives(records: list[dict[str, Any]], *, seed: int = 0) -> list[dict[str, Any]]:
    rng = random.Random(seed)
    hunk_pool = [r["hunk_tokens"] for r in records]
    shuffled_hunks = hunk_pool[:]
    rng.shuffle(shuffled_hunks)
    return [{**r, "hunk_tokens": h} for r, h in zip(records, shuffled_hunks, strict=False)]


def inject_foreign(
    home: list[dict[str, Any]], foreign: list[dict[str, Any]], *, seed: int = 0
) -> list[dict[str, Any]]:
    """Replace hunk_tokens in home records with hunks randomly drawn from a foreign repo."""
    rng = random.Random(seed)
    foreign_hunks = [r["hunk_tokens"] for r in foreign]
    return [{**r, "hunk_tokens": rng.choice(foreign_hunks)} for r in home]


def top_dir(file_path: str) -> str:
    parts = Path(file_path).parts
    return parts[0] if parts else ""


def stratify_scores(
    records: list[dict[str, Any]],
    scores: list[float],
    by: StratifyBy,
) -> dict[str, list[float]]:
    """Group scores by record attribute; returns {group_key: [scores]}."""
    if by == "none":
        return {"all": list(scores)}
    groups: dict[str, list[float]] = {}
    for r, s in zip(records, scores, strict=True):
        if by == "language":
            key = r.get("language", "unknown")
        elif by == "top-dir":
            key = top_dir(r.get("file_path", "")) or "unknown"
        else:
            raise ValueError(f"unknown stratify-by: {by}")
        groups.setdefault(key, []).append(s)
    return groups


def compute_percentiles(scores: list[float]) -> dict[str, float]:
    arr = np.array(scores)
    return {
        "min": float(np.min(arr)),
        "p25": float(np.percentile(arr, 25)),
        "median": float(np.median(arr)),
        "p75": float(np.percentile(arr, 75)),
        "p95": float(np.percentile(arr, 95)),
        "max": float(np.max(arr)),
    }


def compute_auc(good_scores: list[float], bad_scores: list[float]) -> float:
    labels = [0] * len(good_scores) + [1] * len(bad_scores)
    scores = good_scores + bad_scores
    return float(roc_auc_score(labels, scores))


def _vectorize_tfidf(bundle: ModelBundle, texts: list[str]) -> torch.Tensor:
    return torch.tensor(bundle.vectorizer.transform(texts).toarray(), dtype=torch.float32)


def _vectorize_token_embed(bundle: ModelBundle, texts: list[str]) -> torch.Tensor:
    # bundle.vectorizer holds a Vocab object for token_embed encoder
    vocab = bundle.vectorizer  # Vocab stored in TfidfVectorizer slot (sklearn untyped)
    if not isinstance(vocab, Vocab):
        raise TypeError(f"expected Vocab, got {type(vocab)}")
    seq_len = 256
    ids_list = [vocab.encode(text.split())[:seq_len] for text in texts]
    out = torch.zeros(len(ids_list), seq_len, dtype=torch.long)
    for i, ids in enumerate(ids_list):
        if ids:
            out[i, : len(ids)] = torch.tensor(ids, dtype=torch.long)
    return out


def _vectorize_bpe(bundle: ModelBundle, texts: list[str]) -> torch.Tensor:
    bpe_vocab = bundle.vectorizer
    if not isinstance(bpe_vocab, BpeVocab):
        raise TypeError(f"expected BpeVocab, got {type(bpe_vocab)}")
    seq_len = 256
    ids_list = [bpe_vocab.encode(text.split())[:seq_len] for text in texts]
    out = torch.zeros(len(ids_list), seq_len, dtype=torch.long)
    for i, ids in enumerate(ids_list):
        if ids:
            out[i, : len(ids)] = torch.tensor(ids, dtype=torch.long)
    return out


def _vectorize_transformer(bundle: ModelBundle, texts: list[str]) -> torch.Tensor:
    raise NotImplementedError("transformer vectorization not yet implemented")


def _vectorize_pretrained(bundle: ModelBundle, texts: list[str]) -> torch.Tensor:
    pretrained = bundle.vectorizer
    if not isinstance(pretrained, PretrainedEncoder):
        raise TypeError(f"expected PretrainedEncoder, got {type(pretrained)}")
    return pretrained.encode_texts(texts).cpu()


def _vectorize(bundle: ModelBundle, texts: list[str]) -> torch.Tensor:
    if bundle.encoder_kind in ("tfidf", "word_ngrams"):
        return _vectorize_tfidf(bundle, texts)  # word_ngrams reuses same sklearn vectorizer
    elif bundle.encoder_kind == "token_embed":
        return _vectorize_token_embed(bundle, texts)
    elif bundle.encoder_kind == "bpe":
        return _vectorize_bpe(bundle, texts)
    elif bundle.encoder_kind == "pretrained":
        return _vectorize_pretrained(bundle, texts)
    elif bundle.encoder_kind == "transformer":
        return _vectorize_transformer(bundle, texts)
    else:
        raise ValueError(f"unknown encoder_kind: {bundle.encoder_kind!r}")


_SCORE_BATCH = 128


def score_records(
    bundle: ModelBundle,
    records: list[dict[str, Any]],
    *,
    aggregation: Literal["mean", "topk", "random_topk"] = "mean",
    topk_k: int = 64,
    zscore_ref_stats: tuple[float, float] | None = None,
) -> list[float]:
    if not records:
        return []
    ctx_texts = [" ".join(t["text"] for t in r["context_before"]) for r in records]
    hunk_texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in records]
    ctx_x = _vectorize(bundle, ctx_texts)
    hunk_x = _vectorize(bundle, hunk_texts)
    bundle.model.eval()
    scores: list[float] = []
    with torch.no_grad():
        for start in range(0, len(records), _SCORE_BATCH):
            end = start + _SCORE_BATCH
            if aggregation == "topk":
                batch_scores = bundle.model.surprise_topk(
                    ctx_x[start:end], hunk_x[start:end], k=topk_k
                )
            elif aggregation == "random_topk":
                model = bundle.model
                z_ctx = model.encode(ctx_x[start:end])
                z_hunk = model.encode(hunk_x[start:end])
                z_pred = model.predict(z_ctx)
                per_dim = F.mse_loss(z_pred, z_hunk, reduction="none")
                dim = per_dim.shape[-1]
                perm = torch.randperm(dim)[:topk_k]
                batch_scores = per_dim[:, perm].mean(dim=-1)
            else:
                batch_scores = bundle.model.surprise(ctx_x[start:end], hunk_x[start:end])
            scores.extend(batch_scores.detach().cpu().tolist())
    if zscore_ref_stats is not None:
        ref_mean, ref_std = zscore_ref_stats
        scores = [(s - ref_mean) / (ref_std + 1e-8) for s in scores]
    return scores


def _print_table(rows: list[tuple[str, dict[str, float]]]) -> None:
    keys = ["min", "p25", "median", "p75", "p95", "max"]
    header = f"{'':30s}" + "".join(f"{k:>10s}" for k in keys) + f"{'AUC':>8s}"
    print("\n=== Score Distribution ===")
    print(header)
    print("-" * len(header))
    for label, p, auc in rows:  # type: ignore[misc]
        row = f"{label:30s}" + "".join(f"{p[k]:10.4f}" for k in keys) + f"{auc:8.4f}"
        print(row)
    print()


def main() -> None:
    parser = argparse.ArgumentParser(description="Validate argot model on train/held-out split")
    parser.add_argument("--dataset", default=".argot/training.jsonl")
    parser.add_argument("--epochs", type=int, default=20)
    parser.add_argument("--batch-size", type=int, default=128)
    parser.add_argument("--ratio", type=float, default=0.8)
    parser.add_argument(
        "--stratify-by",
        choices=["none", "language", "top-dir"],
        default="none",
        help="Group held-out percentile tables by file attribute",
    )
    parser.add_argument(
        "--dump-scores",
        default=None,
        help="Write held-out per-record scores to this JSONL path",
    )
    args = parser.parse_args()

    dataset_path = Path(args.dataset)
    if not dataset_path.exists():
        print(f"error: dataset not found at {dataset_path}", file=sys.stderr)
        sys.exit(2)

    records = [json.loads(line) for line in dataset_path.read_text().splitlines() if line.strip()]
    if len(records) < 10:
        print("error: dataset too small for validation", file=sys.stderr)
        sys.exit(2)

    print(f"Loaded {len(records)} records from {dataset_path}")

    # Partition by repo — smallest becomes the foreign (unseen) test set
    repo_groups: dict[str, list[dict[str, Any]]] = {}
    for r in records:
        repo_groups.setdefault(r.get("_repo", "unknown"), []).append(r)

    has_cross_repo = len(repo_groups) >= 2
    if has_cross_repo:
        foreign_name = min(repo_groups, key=lambda n: len(repo_groups[n]))
        foreign = repo_groups[foreign_name]
        home = [r for r in records if r.get("_repo") != foreign_name]
        print(
            f"Repos: {list(repo_groups)} — using '{foreign_name}'"
            f" ({len(foreign)} records) as foreign set"
        )
    else:
        home = records
        foreign = []

    train_records, held_out = split_by_time(home, ratio=args.ratio)
    print(f"Split: {len(train_records)} train / {len(held_out)} held-out")

    print(f"\nTraining on {len(train_records)} records ({args.epochs} epochs)...")
    bundle = train_model(train_records, epochs=args.epochs, batch_size=args.batch_size)

    print("\nScoring...")
    good_scores = score_records(bundle, held_out)
    shuffled_scores = score_records(bundle, shuffle_negatives(held_out, seed=42))
    cross_scores = score_records(bundle, foreign) if has_cross_repo else []
    injected_scores = (
        score_records(bundle, inject_foreign(held_out, foreign, seed=42)) if has_cross_repo else []
    )

    good_p = compute_percentiles(good_scores)
    shuffled_auc = compute_auc(good_scores, shuffled_scores)
    rows: list[Any] = [
        ("Good (held-out)", good_p, 0.5),
        ("Bad — shuffled tokens", compute_percentiles(shuffled_scores), shuffled_auc),
    ]
    if has_cross_repo:
        cross_auc = compute_auc(good_scores, cross_scores)
        injected_auc = compute_auc(good_scores, injected_scores)
        rows += [
            ("Bad — cross-repo foreign", compute_percentiles(cross_scores), cross_auc),
            ("Bad — injected foreign hunk", compute_percentiles(injected_scores), injected_auc),
        ]

    _print_table(rows)

    if args.stratify_by != "none":
        strata = stratify_scores(held_out, good_scores, args.stratify_by)
        strata_rows: list[Any] = [
            (f"  {key} (n={len(vals)})", compute_percentiles(vals), 0.5)
            for key, vals in sorted(strata.items())
            if vals
        ]
        print(f"=== Held-out stratified by {args.stratify_by} ===")
        _print_table(strata_rows)

    if args.dump_scores:
        dump_path = Path(args.dump_scores)
        dump_path.parent.mkdir(parents=True, exist_ok=True)
        with dump_path.open("w") as fh:
            for record, score in zip(held_out, good_scores, strict=True):
                fh.write(
                    json.dumps(
                        {
                            "file_path": record.get("file_path", ""),
                            "language": record.get("language", ""),
                            "score": score,
                        }
                    )
                )
                fh.write("\n")
        print(f"held-out scores written to {dump_path}")

    good_median = good_p["median"]
    checks: dict[str, tuple[bool, float]] = {
        "shuffled": (
            compute_percentiles(shuffled_scores)["median"] > good_median,
            shuffled_auc,
        ),
    }
    if has_cross_repo:
        checks["cross-repo"] = (
            compute_percentiles(cross_scores)["median"] > good_median,
            cross_auc,
        )
        checks["injected"] = (
            compute_percentiles(injected_scores)["median"] > good_median,
            injected_auc,
        )

    all_pass = True
    for name, (sep, auc) in checks.items():
        icon = "✓" if sep else "✗"
        auc_icon = "✓" if auc > 0.6 else "✗"
        auc_label = "ok" if auc > 0.6 else "FAIL — below 0.6"
        sep_label = "ok" if sep else "FAIL"
        print(
            f"  {icon} [{name}] median separation {sep_label}"
            f"   {auc_icon} AUC={auc:.4f} ({auc_label})"
        )
        if not sep or auc <= 0.6:
            all_pass = False

    print()
    if all_pass:
        print("✓ All validation checks passed")
    else:
        print("✗ One or more validation checks failed")
        sys.exit(1)


if __name__ == "__main__":
    main()
