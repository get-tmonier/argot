# Investigation Prompt — Static Chunk Audit: Further Improvements

## Context

The static chunk audit achieves delta = 0.2847 on FastAPI, beating the Phase 7 git-history winner (0.2291) by +24%. Training data: 3868 AST-extracted chunks from FastAPI HEAD. Encoder: `microsoft/unixcoder-base` (raw embeddings, `NORMALIZE_EMBEDDINGS=False`). No git history required.

Two variables changed simultaneously vs the baseline (encoder + training data source), so the attribution is unresolved. Key structural issue: top-20 anomalies are 15/20 test files, 0/20 fastapi/ core — the predictor correctly identifies test files as distributional outliers but this creates false-positive noise for PR auditing.

All research scripts live in `engine/argot/research/`. The audit test is `static_chunk_audit_test.py`. Phase 7 sweep results are documented in `docs/research/scoring/signal/phase7_jepa_predictor_finetuning.md`.

---

## Questions to Investigate

### 1. Attribution: encoder vs training data (highest priority)

Run `static_chunk_audit_test.py` with `ENCODER_MODEL = "Salesforce/coderankembed"` (swapping back to the Phase 7 encoder while keeping the static-chunk training data). Compare the resulting delta to:
- Static audit with UnixCoder: 0.2847
- Git-history baseline with CodeRankEmbed: 0.1817
- Phase 7 winner with CodeRankEmbed: 0.2291

This isolates whether the gain comes from the encoder, the training data, or both. Report the delta and whether top-20 composition changes.

### 2. Core-only scoring

The test-file bias in top-20 is a structural property of training on the full repo. Two sub-questions:

**2a. Train on core only, score everything.** Strip `tests/` and `scripts/` from the training corpus, retrain on `fastapi/` only (~X chunks). Does delta change? Does top-20 shift toward framework core? Does delta hold up on the fixture set (which tests breaks in core code)?

**2b. Train on everything, display core only.** Keep current training but filter the ranked output to `fastapi/` paths. What is the effective delta among core-only chunks? Report corpus (core-only) mean, p95, and fixture delta on that filtered subset.

### 3. Cross-repo generalization (zero-shot)

Can a predictor trained on FastAPI static chunks score ky or httpx code without retraining? Run the standard Phase 7 validation (`validate.py` with ky/httpx fixture sets) using a predictor trained on FastAPI static chunks. Report delta for ky and httpx. If generalization holds, the static-chunk approach could train once and apply to multiple repos.

### 4. Static chunk count vs delta curve

The current run uses all 3868 chunks (with 80/20 file holdout → ~3095 train). Does more training data help? Extract additional repos (e.g. starlette, pydantic — close style neighbors of FastAPI) and add their static chunks to the training corpus. Grid: [3095, 6000, 10000] training chunks, all evaluated on FastAPI fixtures. Report delta vs training corpus size. This tests whether the corpus-size bottleneck identified in Phase 7 (at 2000 git records) also applies to static chunks.

### 5. Holdout strategy comparison

Current: file-level 80/20 holdout (whole files in or out). Alternative: function-level random 80/20 (any function can end up in either split). Run both on the same corpus, report delta. If function-level holdout gives higher delta without obvious contamination, it may be preferable (more training data per file). Report top-20 composition change as a contamination proxy.

### 6. Encoder sweep (if time allows)

Other candidates worth trying with `NORMALIZE_EMBEDDINGS=False`:
- `microsoft/codebert-base` (older, 768-dim)
- `Salesforce/codet5p-110m-embedding` (110M, 256-dim — much faster)

For each: report delta and training time. The goal is finding the best delta/speed tradeoff for the static audit use case.

---

## Evaluation Protocol

For all runs:
- Primary metric: **delta = mean(break scores) − mean(control scores)** on FastAPI fixtures
- Secondary: top-20 breakdown (% from fastapi/ core vs tests/ vs docs/)
- Use the existing `static_chunk_audit_test.py` infrastructure; add a config table at the top if running a sweep
- Single seed is fine (the ensemble already collapses variance to ~0.000)
- Report against three anchors: static-UnixCoder (0.2847), Phase 7 winner (0.2291), git-history baseline (0.1817)

## Success criteria

- Q1 attribution resolved with a clean data point
- At least one approach that improves top-20 core-file representation without sacrificing delta
- Corpus-size curve started (even just 2 points)
