"""Quick experiment: Qwen2.5-Coder-1.5B-Instruct as re-ranker on FastAPI fixtures.

Scores all 27 fixtures with both JEPA and the LLM, reports AUC at every
blend weight so we can see how much the LLM adds.

Tensor encoding is cached under /tmp/argot-llm-rerank-cache/ keyed by
(HEAD sha, encoder model) — the slow 4-min UnixCoder pass only runs once.
Second run: load cache → train JEPA (~10s) → score LLM (~1min) → done.

Run: uv run python -m argot.research.llm_rerank_experiment
"""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any

import numpy as np
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

from argot.acceptance.runner import CATALOG_DIR, FixtureSpec, fixture_to_record, load_manifest
from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.static_chunk_audit_test import (
    ENCODER_MODEL,
    ENSEMBLE_N,
    FASTAPI_CLONE_DIR,
    NORMALIZE_EMBEDDINGS,
    WINNER_BETA,
    WINNER_TAU,
    WINNER_WARMUP,
    _clone_or_reuse,
    _EnsembleForAudit,
    _extract_chunks,
)
from argot.train import _texts_for_records
from argot.validate import compute_auc

LLM_MODEL = "Qwen/Qwen2.5-Coder-1.5B-Instruct"
ENTRY_DIR = CATALOG_DIR / "fastapi"
_REPO_ROOT = Path(__file__).parent.parent.parent.parent
_CACHE_DIR = Path("/tmp/argot-llm-rerank-cache")

_SYSTEM = "You are a code reviewer for a FastAPI codebase."

_USER = """\
Below is a Python snippet from a pull request against a FastAPI codebase.

FastAPI idiomatic patterns:
- @app.get / @router.post with type-annotated parameters
- Pydantic BaseModel for validation (never manual isinstance checks)
- HTTPException for all errors (never ValueError, RuntimeError, KeyError)
- Depends() for dependency injection
- async def for I/O-bound endpoints
- httpx + response.raise_for_status() for downstream HTTP calls

```python
{code}
```

Reply with exactly one word: IDIOMATIC or FOREIGN"""


# ---------------------------------------------------------------------------
# Tensor cache
# ---------------------------------------------------------------------------


def _cache_key(head_sha: str) -> str:
    model_slug = ENCODER_MODEL.replace("/", "_")
    return f"{head_sha[:12]}_{model_slug}"


def _save_enc(key: str, label: str, ctx: torch.Tensor, hunk: torch.Tensor) -> None:
    _CACHE_DIR.mkdir(exist_ok=True)
    torch.save(ctx, _CACHE_DIR / f"{key}_{label}_ctx.pt")
    torch.save(hunk, _CACHE_DIR / f"{key}_{label}_hunk.pt")
    print(f"  Cached {label} tensors → {_CACHE_DIR}", flush=True)


def _load_enc(key: str, label: str) -> tuple[torch.Tensor, torch.Tensor] | None:
    ctx_p = _CACHE_DIR / f"{key}_{label}_ctx.pt"
    hunk_p = _CACHE_DIR / f"{key}_{label}_hunk.pt"
    if ctx_p.exists() and hunk_p.exists():
        ctx = torch.load(ctx_p, weights_only=True)
        hunk = torch.load(hunk_p, weights_only=True)
        print(f"  Loaded {label} tensors from cache ({ctx.shape[0]} records)", flush=True)
        return ctx, hunk
    return None


def _encode(
    records: list[dict[str, Any]],
    key: str,
    label: str,
) -> tuple[torch.Tensor, torch.Tensor]:
    cached = _load_enc(key, label)
    if cached is not None:
        return cached
    device = select_device()
    enc = PretrainedEncoder(device=device, model_name=ENCODER_MODEL)
    ctx_texts, hunk_texts = _texts_for_records(records)
    n = len(records)
    print(f"  Encoding {n} {label} ({n * 2} texts) with {ENCODER_MODEL} ...", flush=True)
    t0 = time.perf_counter()
    with torch.no_grad():
        ctx_x = enc.encode_texts(ctx_texts, normalize_embeddings=NORMALIZE_EMBEDDINGS).cpu()
        hunk_x = enc.encode_texts(hunk_texts, normalize_embeddings=NORMALIZE_EMBEDDINGS).cpu()
    del enc
    print(f"  done in {time.perf_counter() - t0:.1f}s", flush=True)
    _save_enc(key, label, ctx_x, hunk_x)
    return ctx_x, hunk_x


# ---------------------------------------------------------------------------
# Fixture source
# ---------------------------------------------------------------------------


def _fixture_source(spec: FixtureSpec) -> str:
    """Read the raw source lines for a fixture hunk."""
    path = ENTRY_DIR / spec.file
    lines = path.read_text(encoding="utf-8").splitlines()
    start = max(0, spec.hunk_start_line - 1)
    end = spec.hunk_end_line
    return "\n".join(lines[start:end])


# ---------------------------------------------------------------------------
# LLM scoring — P(FOREIGN) from next-token logits
# ---------------------------------------------------------------------------


def _load_llm(device: str) -> tuple[Any, Any]:
    print(f"\nLoading {LLM_MODEL} ...", flush=True)
    t0 = time.perf_counter()
    tokenizer = AutoTokenizer.from_pretrained(LLM_MODEL)  # type: ignore[no-untyped-call]
    model = AutoModelForCausalLM.from_pretrained(
        LLM_MODEL,
        dtype=torch.float16,
        device_map=device,
    )
    model.eval()  # type: ignore[no-untyped-call]
    print(f"  loaded in {time.perf_counter() - t0:.1f}s", flush=True)
    return model, tokenizer


def _p_foreign(model: Any, tokenizer: Any, code: str, device: str) -> float:
    """P(FOREIGN) / (P(FOREIGN) + P(IDIOMATIC)) from the model's next-token distribution."""
    messages = [
        {"role": "system", "content": _SYSTEM},
        {"role": "user", "content": _USER.format(code=code)},
    ]
    text = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
    inputs = tokenizer(text, return_tensors="pt", truncation=True, max_length=3072).to(device)
    with torch.no_grad():
        logits = model(**inputs).logits[0, -1]  # next-token distribution

    # Use first sub-word token of each label word
    f_id = tokenizer.encode("FOREIGN", add_special_tokens=False)[0]
    i_id = tokenizer.encode("IDIOMATIC", add_special_tokens=False)[0]
    pair = torch.softmax(logits[[f_id, i_id]], dim=0)
    return float(pair[0])


# ---------------------------------------------------------------------------
# Metrics
# ---------------------------------------------------------------------------


def _blend_auc_table(
    specs: list[FixtureSpec],
    jepa: list[float],
    llm: list[float],
) -> list[tuple[str, float]]:
    jepa_arr = np.array(jepa)
    llm_arr = np.array(llm)
    jepa_z = (jepa_arr - jepa_arr.mean()) / (jepa_arr.std() + 1e-9)

    rows = []
    for alpha, label in [
        (1.00, "JEPA only         (α=1.00)"),
        (0.75, "75% JEPA + 25% LLM (α=0.75)"),
        (0.50, "50% JEPA + 50% LLM (α=0.50)"),
        (0.25, "25% JEPA + 75% LLM (α=0.25)"),
        (0.00, "LLM only          (α=0.00)"),
    ]:
        combined = alpha * jepa_z + (1.0 - alpha) * llm_arr
        breaks = [float(combined[i]) for i, s in enumerate(specs) if s.is_break]
        ctrls = [float(combined[i]) for i, s in enumerate(specs) if not s.is_break]
        rows.append((label, compute_auc(ctrls, breaks)))
    return rows


def _category_auc_table(
    specs: list[FixtureSpec],
    jepa: list[float],
    llm: list[float],
) -> list[tuple[str, float, float]]:
    """Per-category AUC: JEPA vs LLM (skip categories with only one class)."""
    cats: dict[str, list[tuple[float, float, bool]]] = {}
    for s, j, lv in zip(specs, jepa, llm, strict=False):
        cats.setdefault(s.category, []).append((j, lv, s.is_break))

    rows = []
    for cat in sorted(cats):
        entries = cats[cat]
        if not any(e[2] for e in entries) or all(e[2] for e in entries):
            continue
        j_br = [e[0] for e in entries if e[2]]
        j_ct = [e[0] for e in entries if not e[2]]
        l_br = [e[1] for e in entries if e[2]]
        l_ct = [e[1] for e in entries if not e[2]]
        rows.append((cat, compute_auc(j_ct, j_br), compute_auc(l_ct, l_br)))
    return rows


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------


def _print_results(
    specs: list[FixtureSpec],
    jepa: list[float],
    llm: list[float],
) -> None:
    blend_rows = _blend_auc_table(specs, jepa, llm)
    cat_rows = _category_auc_table(specs, jepa, llm)
    best_label, best_auc = max(blend_rows, key=lambda r: r[1])

    print("\n=== LLM Re-rank Experiment — AUC by blend weight ===")
    for label, auc in blend_rows:
        flag = " ✅" if auc >= 0.80 else ""
        print(f"  {label}  AUC={auc:.4f}{flag}")
    print(f"\n  Best: {best_label.strip()}  AUC={best_auc:.4f}")

    print("\n=== Per-category AUC (JEPA vs LLM) ===")
    print(f"  {'category':25}  {'JEPA':>6}  {'LLM':>6}  {'gain':>6}")
    for cat, j, lv in cat_rows:
        gain = lv - j
        flag = "  ✅" if gain > 0.05 else ("  ⚠️ " if gain < -0.05 else "")
        print(f"  {cat:25}  {j:.4f}  {lv:.4f}  {gain:+.4f}{flag}")

    print("\n=== Per-fixture P(FOREIGN) ===")
    print(f"  {'fixture':48}  {'truth':>5}  {'P(FOR)':>6}  {'JEPA':>6}")
    for s, j, lv in sorted(zip(specs, jepa, llm, strict=False), key=lambda x: -x[2]):
        truth = "BREAK" if s.is_break else "CTRL"
        print(f"  {s.name:48}  {truth:>5}  {lv:.3f}   {j:.4f}")


def _write_report(
    specs: list[FixtureSpec],
    jepa: list[float],
    llm: list[float],
    head_sha: str,
) -> None:
    blend_rows = _blend_auc_table(specs, jepa, llm)
    cat_rows = _category_auc_table(specs, jepa, llm)
    best_label, best_auc = max(blend_rows, key=lambda r: r[1])

    md = [
        "# Phase 8 LLM Re-rank Experiment — 2026-04-20",
        "",
        "## Setup",
        "",
        f"- JEPA encoder: `{ENCODER_MODEL}`",
        f"- LLM: `{LLM_MODEL}`",
        f"- FastAPI HEAD: `{head_sha}`",
        f"- Fixtures: {len(specs)} ({sum(s.is_break for s in specs)} breaks, "
        f"{sum(not s.is_break for s in specs)} controls)",
        "- Command: `uv run python -m argot.research.llm_rerank_experiment`",
        "",
        "## AUC by Blend Weight",
        "",
        "| Approach | AUC |",
        "|----------|----:|",
    ]
    for label, auc in blend_rows:
        md.append(f"| {label.strip()} | {auc:.4f} |")

    md += [
        "",
        f"**Best: {best_label.strip()} — AUC {best_auc:.4f}**",
        "",
        "## Per-category AUC",
        "",
        "| Category | JEPA | LLM | Gain |",
        "|----------|-----:|----:|-----:|",
    ]
    for cat, j, lv in cat_rows:
        md.append(f"| {cat} | {j:.4f} | {lv:.4f} | {lv - j:+.4f} |")

    report_dir = _REPO_ROOT / "docs" / "research" / "scoring" / "signal"
    report_dir.mkdir(parents=True, exist_ok=True)
    path = report_dir / "phase8_llm_rerank_2026-04-20.md"
    path.write_text("\n".join(md) + "\n", encoding="utf-8")
    print(f"\nReport written to {path}", flush=True)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    t_start = time.perf_counter()
    device = (
        "mps"
        if torch.backends.mps.is_available()
        else ("cuda" if torch.cuda.is_available() else "cpu")
    )

    # 1. Clone/reuse + cache key
    head_sha = _clone_or_reuse()
    key = _cache_key(head_sha)

    # 2. Extract chunks and encode (cached after first run)
    print("\nExtracting static chunks ...", flush=True)
    chunks = _extract_chunks(FASTAPI_CLONE_DIR, head_sha)
    print(f"  {len(chunks)} chunks", flush=True)

    print("\nEncoding (chunks):", flush=True)
    chunk_enc = _encode(chunks, key, "chunks")

    specs = load_manifest(ENTRY_DIR)
    fixture_records = [fixture_to_record(ENTRY_DIR, s) for s in specs]
    print("\nEncoding (fixtures):", flush=True)
    fix_enc = _encode(fixture_records, key, "fixtures")

    # 3. Train JEPA on core-only, score fixtures
    core_idx = [i for i, c in enumerate(chunks) if c["_file_class"] != "test"]
    core_chunks = [chunks[i] for i in core_idx]
    idx_t = torch.tensor(core_idx, dtype=torch.long)
    core_enc = (chunk_enc[0][idx_t], chunk_enc[1][idx_t])

    print(f"\nTraining JEPA on {len(core_chunks)} core chunks ...", flush=True)
    ensemble = _EnsembleForAudit(
        n=ENSEMBLE_N,
        beta=WINNER_BETA,
        tau=WINNER_TAU,
        warmup_epochs=WINNER_WARMUP,
    )
    ensemble.fit(core_chunks, preencoded=core_enc)

    print(f"Scoring {len(specs)} fixtures with JEPA ...", flush=True)
    jepa_scores = ensemble.score_from_preencoded(*fix_enc)

    # 4. Score fixtures with LLM
    model, tokenizer = _load_llm(device)
    print(f"\nScoring {len(specs)} fixtures with LLM (P(FOREIGN)) ...", flush=True)
    llm_scores: list[float] = []
    for i, spec in enumerate(specs):
        code = _fixture_source(spec)
        score = _p_foreign(model, tokenizer, code, device)
        truth = "BREAK" if spec.is_break else "CTRL "
        print(f"  [{i + 1:2}/{len(specs)}] {truth}  P(FOREIGN)={score:.3f}  {spec.name}")
        llm_scores.append(score)

    elapsed = time.perf_counter() - t_start
    print(f"\nTotal elapsed: {elapsed:.0f}s", flush=True)

    _print_results(specs, jepa_scores, llm_scores)
    _write_report(specs, jepa_scores, llm_scores, head_sha)


if __name__ == "__main__":
    main()
