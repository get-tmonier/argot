# Static Chunk Audit — Feasibility Study

**Date:** 2026-04-20  
**Branch:** research/phase-7-honest-eval  
**Script:** `engine/argot/research/static_chunk_audit_test.py`

## Motivation

The current scorer (HoldoutPartitionJepaScorer) depends on commit history: it trains on commit hunks and scores commit hunks. For an **audit mode** — pointing the tool at any repo checkout and surfacing anomalies — we need something that works from a static snapshot with no git history.

Question: if we replace commit hunks with AST-extracted function/class bodies from the current HEAD, does JEPA surprise scoring still produce usable signal?

## Setup

- **Target repo:** FastAPI (`tiangolo/fastapi`, HEAD `2fa00db`)
- **Encoder:** `nomic-ai/CodeRankEmbed` (frozen, `max_seq_length=512`)
- **Scorer:** 3-member `JepaInfoNCEScorer` ensemble, winner config `b01_t01_w0` (β=0.1, τ=0.1, warmup=0)
- **Training corpus:** AST-extracted static chunks from FastAPI HEAD (no commit history)
- **Extraction:** `FunctionDef`, `AsyncFunctionDef`, `ClassDef` ≥ 3 lines; parent must be `Module` or `ClassDef` (no nested functions)
- **Context:** full preceding file content — encoder sees first 512 tokens = file header (imports, declarations)

## Runs

### Run 1 — Baseline static (full context, dedup nested)

| metric | value |
|---|---|
| chunks extracted | 3868 |
| corpus mean | 0.4702 |
| corpus p95 | 0.9404 |
| fixture break mean | 1.2665 |
| fixture ctrl mean | 1.0848 |
| **delta** | **0.1817** |

Commit-hunk baseline for reference: break=1.2265, ctrl=1.1333, **delta=0.0932**

**Static chunks beat commit-hunk delta by 2×.** The top-20 is entirely named Python units (functions/classes with precise line ranges). No JS files, no Jinja templates, no anonymous diff fragments. Bottom-5 are all identical `class Item(BaseModel)` boilerplate from `docs_src/` tutorials — correctly rated as maximally "normal."

Notable top-20 hits:
- `scripts/people.py::update_content` — YAML dump pattern foreign to FastAPI web style
- `fastapi/dependencies/utils.py::ensure_multipart_is_installed` — unusual try/except/import nesting
- `fastapi/routing.py::_keepalive_inserter` — async SSE streaming (rare in codebase)
- `fastapi/security/utils.py::get_authorization_scheme_param` — minimal utility with no framework idioms

### Run 2 — 50-line context window + nested dedup (hypothesis: fix context truncation)

Hypothesis: CodeRankEmbed's 512-token cap means the encoder sees the middle of the file for deep functions, losing imports. Fix: last 50 lines before chunk start.

| metric | value |
|---|---|
| chunks extracted | 3868 |
| corpus mean | 0.3941 |
| corpus p95 | 0.8378 |
| fixture break mean | 1.2223 |
| fixture ctrl mean | 1.1599 |
| **delta** | **0.0624** |

**Worse.** Delta collapsed from 0.1817 → 0.0624.

## Key Finding: Context Truncation Intuition Was Wrong

sentence-transformers truncates from the **right** (keeps first `max_seq_length` tokens). So with full preceding file content, the encoder always sees the **file header** (imports, module docstring, class declarations at top of file) — the most discriminative context for style. Stripping to 50 lines replaced that with a narrow window of mid-file code, losing the structural signal entirely.

**Rule: keep full preceding file as context.** The truncation is doing the right thing.

## Lessons

| hypothesis | result |
|---|---|
| 50-line context window improves signal | ✗ Hurt delta by 65% |
| Nested dedup (Module/ClassDef parents only) | Neutral (3868 chunks either way) |
| Static chunks can replace commit hunks | ✓ Better delta (0.1817 vs 0.0932) |
| Bottom-5 = obvious boilerplate | ✓ Confirmed (identical Item classes) |

## Next Experiment

Three changes to test together:

1. **Encoder swap to UnixCoder** (`microsoft/unixcoder-base`) — structure-aware, trained on code understanding not retrieval; needs custom SentenceTransformer wrapper + `normalize_embeddings=True`
2. **File-level holdout split** — currently JepaInfoNCEScorer splits 80/20 by chunk index, so methods from the same file land in both train and test; split by file instead so the scorer hasn't seen any chunk from held-out files during training
3. **normalize_embeddings=True** — UnixCoder is cosine-similarity-trained; MSE on unnormalized embeddings is undefined relative to what the model expects

Keep: full preceding file context, Module/ClassDef parent filter.

Rationale for file-level holdout being highest-impact: UnixCoder encodes structural idioms precisely, so within-file contamination (train/test from same file) would let the predictor memorize per-file style rather than learning cross-file patterns. Genuine OOD signal requires the test chunks to come from files the model has never seen.
