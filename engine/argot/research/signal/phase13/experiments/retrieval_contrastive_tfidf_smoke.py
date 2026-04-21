# engine/argot/research/signal/phase13/experiments/retrieval_contrastive_tfidf_smoke.py
"""Phase 13: Retrieval-Augmented Contrastive TF-IDF smoke test (v2, filtered).

Hypothesis: replacing global P_A (marginal over all repo hunks) with local P_A
(marginal over the top-k most similar hunks retrieved by embedding similarity)
fixes the click ceiling caused by sparse held-out identifiers producing false-positive
log-ratios.

Stage 1 only — score two FastAPI fixtures (routing break vs routing control) and
report delta. Stop and wait for user approval before Stage 2.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/\\
            retrieval_contrastive_tfidf_smoke.py \\
        --out docs/research/scoring/signal/phase13/experiments/\\
            retrieval_contrastive_tfidf_smoke_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
import math
import re
from collections import Counter
from pathlib import Path
from typing import Any

import numpy as np
import torch

from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device

_EPSILON = 1e-7
_MODEL_NAME = "microsoft/unixcoder-base"
_K = 20
_FASTAPI_CORPUS = (
    Path(__file__).parent.parent.parent.parent.parent
    / "acceptance"
    / "catalog"
    / "fastapi"
    / "corpus_file_only.jsonl"
)
_FASTAPI_FIXTURES = (
    Path(__file__).parent.parent.parent.parent.parent
    / "acceptance"
    / "catalog"
    / "fastapi"
    / "fixtures"
    / "default"
)
_REFERENCE_PATH = (
    Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens_bpe.json"
)

_SMOKE_FIXTURES = [
    "paradigm_break_flask_routing",
    "control_router_endpoint",
]


def _is_meaningful_token(token: str) -> bool:
    stripped = token[2:] if token.startswith("##") else token
    stripped = stripped.strip()
    return len(stripped) >= 3 and bool(re.search(r"[A-Za-z0-9]", stripped))


def _hunk_text_from_record(record: dict[str, Any]) -> str:
    if "hunk_source" in record:
        return str(record["hunk_source"])
    return " ".join(t["text"] for t in record.get("hunk_tokens", []))


def _load_corpus(path: Path, limit: int | None = None) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as fh:
        for line in fh:
            records.append(json.loads(line))
            if limit is not None and len(records) >= limit:
                break
    return records


def _load_model_b() -> tuple[dict[int, int], int]:
    raw: dict[str, Any] = json.loads(_REFERENCE_PATH.read_text(encoding="utf-8"))
    token_counts: dict[int, int] = {int(k): v for k, v in raw["token_counts"].items()}
    total_tokens: int = raw["total_tokens"]
    return token_counts, total_tokens


def _build_retrieval_index(
    corpus_texts: list[str],
    encoder: PretrainedEncoder,
) -> np.ndarray:
    """Encode all corpus hunks and return normalized embeddings as (N, D) float32 array."""
    emb: torch.Tensor = encoder.encode_texts(corpus_texts, normalize_embeddings=True)
    arr: np.ndarray = emb.cpu().float().numpy()
    return arr


def _retrieve_top_k(
    query_text: str,
    corpus_texts: list[str],
    index: np.ndarray,
    encoder: PretrainedEncoder,
    k: int = _K,
) -> list[tuple[int, float]]:
    """Return list of (corpus_idx, similarity) for top-k nearest neighbors."""
    q_emb: torch.Tensor = encoder.encode_texts([query_text], normalize_embeddings=True)
    q: np.ndarray = q_emb.cpu().float().numpy()[0]  # (D,)
    sims: np.ndarray = index @ q  # (N,)
    top_k_idx = int(min(k, len(corpus_texts)))
    indices = np.argpartition(sims, -top_k_idx)[-top_k_idx:]
    indices = indices[np.argsort(sims[indices])[::-1]]
    return [(int(i), float(sims[i])) for i in indices]


def _build_local_model_a(
    neighbors: list[tuple[int, float]],
    corpus_texts: list[str],
    tokenizer: Any,
) -> tuple[dict[int, int], int]:
    """Compute Laplace-smoothed token frequency distribution over top-k retrieved hunks."""
    counts: Counter[int] = Counter()
    for idx, _ in neighbors:
        ids: list[int] = tokenizer.encode(corpus_texts[idx], add_special_tokens=False)
        counts.update(ids)
    # Laplace add-1 smoothing: add 1 to each count; vocabulary = all observed tokens
    smoothed: dict[int, int] = {tok: cnt + 1 for tok, cnt in counts.items()}
    total = sum(smoothed.values())
    return smoothed, total


def _fixture_hunk_text(fixture_name: str) -> str:
    path = _FASTAPI_FIXTURES / f"{fixture_name}.py"
    return path.read_text(encoding="utf-8", errors="replace")


def _top3_tokens(ids: list[int], scores: list[float], tokenizer: Any) -> str:
    paired = sorted(zip(scores, ids, strict=True), reverse=True)[:3]
    parts = []
    for s, tok_id in paired:
        tok_str = tokenizer.decode([tok_id]).replace("\n", "\\n").strip()
        parts.append(f"`{tok_str}` ({s:.3f})")
    return " / ".join(parts)


def _score_fixture(
    fixture_name: str,
    corpus_texts: list[str],
    index: np.ndarray,
    encoder: PretrainedEncoder,
    tokenizer: Any,
    model_b: dict[int, int],
    total_b: int,
    k: int = _K,
) -> tuple[float, float, str, list[tuple[int, float]], int, int]:
    """Score one fixture. Returns (max, mean, top3, neighbors, n_tokens, n_filtered)."""
    hunk_text = _fixture_hunk_text(fixture_name)
    neighbors = _retrieve_top_k(hunk_text, corpus_texts, index, encoder, k=k)
    model_a, total_a = _build_local_model_a(neighbors, corpus_texts, tokenizer)

    ids: list[int] = tokenizer.encode(hunk_text, add_special_tokens=False)
    n_tokens = len(ids)

    filtered_ids = [i for i in ids if _is_meaningful_token(tokenizer.decode([i]))]
    n_filtered = len(filtered_ids)

    if not filtered_ids:
        return 0.0, 0.0, "", neighbors, n_tokens, n_filtered

    per_token_scores = [
        math.log(model_b.get(i, 0) / total_b + _EPSILON)
        - math.log(model_a.get(i, 0) / total_a + _EPSILON)
        for i in filtered_ids
    ]
    max_score = max(per_token_scores)
    mean_score = sum(per_token_scores) / len(per_token_scores)
    top3 = _top3_tokens(filtered_ids, per_token_scores, tokenizer)
    return max_score, mean_score, top3, neighbors, n_tokens, n_filtered


def _verdict(delta_max: float, mean_delta: float) -> str:
    if delta_max >= 0.3 and mean_delta > 0:
        return "FILTER RESCUED SIGNAL. Proceed to Stage 2 full run."
    if delta_max < 0.1:
        return (
            "PUNCTUATION WAS THE ONLY SIGNAL. "
            "Retrieval does not capture paradigm. Abandon retrieval direction."
        )
    return (
        "FILTER INSUFFICIENT or RETRIEVAL NOT DISCRIMINATING. "
        "Print the top-10 retrieved neighbors for each fixture and STOP — "
        "user will decide next step from the retrieval diagnostic."
    )


def _neighbor_lines(
    neighbors: list[tuple[int, float]],
    corpus_texts: list[str],
    n: int = 3,
) -> list[str]:
    lines = []
    for rank, (idx, sim) in enumerate(neighbors[:n], 1):
        preview = corpus_texts[idx][:100].replace("\n", " ")
        lines.append(f"  {rank}. sim={sim:.4f}  {preview!r}")
    return lines


def run(
    corpus_path: Path = _FASTAPI_CORPUS,
    out: Path | None = None,
    k: int = _K,
    corpus_limit: int = 2000,
) -> float:
    print("Loading corpus…", flush=True)
    corpus_records = _load_corpus(corpus_path, limit=corpus_limit)
    corpus_texts = [_hunk_text_from_record(r) for r in corpus_records]
    print(f"Loaded {len(corpus_texts)} corpus records.", flush=True)

    print("Loading encoder (UnixCoder)…", flush=True)
    device = select_device()
    encoder = PretrainedEncoder(model_name=_MODEL_NAME, device=device)

    print("Loading BPE tokenizer…", flush=True)
    from transformers import AutoTokenizer  # local import to keep top-level clean

    tokenizer = AutoTokenizer.from_pretrained(_MODEL_NAME)  # type: ignore[no-untyped-call]

    print("Building retrieval index (brute-force cosine)…", flush=True)
    index = _build_retrieval_index(corpus_texts, encoder)

    model_b, total_b = _load_model_b()

    print(f"Scoring {len(_SMOKE_FIXTURES)} fixtures (k={k})…", flush=True)
    results: list[tuple[str, float, float, str, list[tuple[int, float]], int, int]] = []
    for fixture_name in _SMOKE_FIXTURES:
        max_s, mean_s, top3, neighbors, n_tok, n_filt = _score_fixture(
            fixture_name, corpus_texts, index, encoder, tokenizer, model_b, total_b, k=k
        )
        results.append((fixture_name, max_s, mean_s, top3, neighbors, n_tok, n_filt))
        print(
            f"  {fixture_name}: n_tokens={n_tok} n_filtered={n_filt} "
            f"max={max_s:.4f} mean={mean_s:.4f} top3={top3}",
            flush=True,
        )

    break_max, break_mean = results[0][1], results[0][2]
    ctrl_max, ctrl_mean = results[1][1], results[1][2]
    delta_max = break_max - ctrl_max
    delta_mean = break_mean - ctrl_mean

    print("\n=== Retrieval-Augmented Contrastive TF-IDF Smoke v2 (filtered) ===")
    print(f"Retrieval corpus: {corpus_limit} FastAPI records")
    print(f"k={k} nearest neighbors")
    print("Filter: len >= 3 AND has alphanumeric (BPE ## stripped before length check)")
    print()
    header = (
        f"{'Fixture':<35} | {'n_tokens':>8} | {'n_filtered':>10} "
        f"| {'max':>7} | {'mean':>7} | top-3 meaningful tokens"
    )
    sep = "-" * len(header)
    print(header)
    print(sep)
    for name, max_s, mean_s, top3, _, n_tok, n_filt in results:
        print(f"{name:<35} | {n_tok:>8} | {n_filt:>10} | {max_s:>7.3f} | {mean_s:>7.3f} | {top3}")

    print()
    print(f"Delta (break − control, max):  {delta_max:+.3f}")
    print(f"Delta (break − control, mean): {delta_mean:+.3f}")

    verdict = _verdict(delta_max, delta_mean)

    # Always print top-3 neighbor diagnostics; top-10 for Case C
    n_neighbors = 10 if "FILTER INSUFFICIENT" in verdict else 3
    for name, _, _, _, neighbors, _, _ in results:
        print(f"\nTop-{n_neighbors} retrieved neighbors for {name}:")
        for line in _neighbor_lines(neighbors, corpus_texts, n=n_neighbors):
            print(line)

    print(f"\nVerdict: {verdict}")

    if out is not None:
        _write_report(out, results, delta_max, delta_mean, verdict, corpus_texts, k, corpus_limit)

    return delta_max


def _write_report(
    out: Path,
    results: list[tuple[str, float, float, str, list[tuple[int, float]], int, int]],
    delta_max: float,
    delta_mean: float,
    verdict: str,
    corpus_texts: list[str],
    k: int,
    corpus_limit: int,
) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    n_neighbors = 10 if "FILTER INSUFFICIENT" in verdict else 3
    lines: list[str] = [
        "# Phase 13 — Retrieval-Augmented Contrastive TF-IDF Smoke Test v2 (filtered) (2026-04-21)",
        "",
        "## Setup",
        "",
        f"- Retrieval corpus: {corpus_limit} FastAPI records (`corpus_file_only.jsonl`)",
        f"- k = {k} nearest neighbors (cosine similarity, UnixCoder BPE embeddings)",
        "- P_A: Laplace add-1 smoothed token frequencies over top-k retrieved hunks",
        "- P_B: `generic_tokens_bpe.json` (generic code reference)",
        "- Fixtures: routing category (break vs control)",
        "- Token filter: len >= 3 AND has alphanumeric (BPE `##` prefix stripped first)",
        "",
        "## Results",
        "",
        "=== Retrieval-Augmented Contrastive TF-IDF Smoke v2 (filtered) ===",
        f"Retrieval corpus: {corpus_limit} FastAPI records",
        f"k={k} nearest neighbors",
        "Filter: len ≥ 3 AND has alphanumeric (BPE ## stripped before length check)",
        "",
        "| Fixture | n_tokens | n_filtered | max | mean | top-3 meaningful tokens |",
        "|---|---|---|---|---|---|",
    ]
    for name, max_s, mean_s, top3, _, n_tok, n_filt in results:
        lines.append(f"| {name} | {n_tok} | {n_filt} | {max_s:.3f} | {mean_s:.3f} | {top3} |")
    lines += [
        "",
        f"Delta (break − control, max):  {delta_max:+.3f}",
        f"Delta (break − control, mean): {delta_mean:+.3f}",
        "",
        f"**Verdict: {verdict}**",
        "",
        f"## Retrieved Neighbor Diagnostics (top-{n_neighbors})",
        "",
        f"*(Top-{n_neighbors} retrieved neighbors per fixture — key diagnostic)*",
        "",
    ]
    for name, _, _, _, neighbors, _, _ in results:
        lines.append(f"### {name}")
        lines.append("")
        for rank, (idx, sim) in enumerate(neighbors[:n_neighbors], 1):
            preview = corpus_texts[idx][:100].replace("\n", " ").replace("|", "\\|")
            lines.append(f"{rank}. sim={sim:.4f} — `{preview}`")
        lines.append("")
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(
        description="Retrieval-Augmented Contrastive TF-IDF smoke test v2 (filtered)"
    )
    parser.add_argument("--out", help="Path for markdown report (optional)")
    parser.add_argument("--k", type=int, default=_K, help="Number of nearest neighbors")
    parser.add_argument("--limit", type=int, default=2000, help="Max corpus records to load")
    args = parser.parse_args(argv)
    out = Path(args.out) if args.out else None
    run(out=out, k=args.k, corpus_limit=args.limit)


if __name__ == "__main__":
    main()
