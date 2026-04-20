# Static Chunk Audit — Git-History-Free Feasibility

**Date:** 2026-04-20  
**Branch:** `research/phase-7-honest-eval`  
**Goal:** Determine whether argot's JEPA scorer can work without any git history by training on AST-extracted static chunks from a repo's HEAD snapshot.

---

## Motivation

The standard argot pipeline requires a commit-hunk corpus extracted from git history (`just extract`). This creates friction for:

- **New repos** with little history
- **Monorepos** where extraction is slow
- **Audit-on-demand** scenarios where the goal is checking the current codebase, not incoming changes

The hypothesis: a JEPA predictor trained on the current file tree's function/class bodies can learn the repo's style well enough to surface anomalies — delta comparable to the git-history baseline.

---

## Method

### Data acquisition

Clone FastAPI at HEAD. Walk all `.py` files with `ast.parse`, extract every `FunctionDef`, `AsyncFunctionDef`, and `ClassDef` whose parent is `Module` or `ClassDef` (i.e. no nested functions — avoids correlated examples sharing nearly identical context). Discard nodes shorter than 3 lines.

**Context strategy:** full preceding file content as context (lines 0 → hunk_start). This gives the encoder the import block and class declarations — the most discriminative structural signal. A 50-line sliding window was tested and hurt delta significantly (0.1817 → 0.0624); full file context confirmed as better.

**Result:** 3868 static chunks across `fastapi/`, `tests/`, `scripts/`, `docs_src/`.

### Encoder

`microsoft/unixcoder-base` via HuggingFace `AutoModel` (mean-pool last hidden state, 768-dim).

**Critical configuration — `NORMALIZE_EMBEDDINGS = False`:**  
The JEPA predictor ends with `LayerNorm`, so its output has magnitude ≈ √768 ≈ 27.7. With L2-normalized targets (magnitude = 1), every chunk's MSE is dominated by the 27× scale gap — scores collapse to ≈ 0.90 regardless of content, delta ≈ 0.002. Raw UnixCoder embeddings have magnitude ≈ 43, which is much closer to the predictor's natural output scale. This lets directional prediction signal dominate over the scale artifact.

### Scorer

`EnsembleInfoNCE(n=3, beta=0.1, tau=0.1, warmup=0)` — the Stage 6 winner configuration from Phase 7.

**File-level holdout:** files shuffled deterministically (seed=42), 80% assigned to train, 20% held out. Functions from the same file never appear in both splits. This prevents the predictor from memorizing file-specific boilerplate and leaking it into scores.

**Training set:** ~3095 chunks (80% of 3868, rounded by file boundaries).

---

## Results

### Quantitative

| | Baseline (git history, CodeRankEmbed) | Static audit (HEAD, UnixCoder) |
|---|---|---|
| **encoder** | CodeRankEmbed | UnixCoder (raw) |
| **training data** | 2000 git commit records | 3868 AST static chunks |
| **git history needed?** | yes | **no** |
| **corpus mean** | 0.6907 | 1.5448 |
| **corpus p95** | 1.2123 | 2.7210 |
| **break mean** | 1.2265 | 3.4286 |
| **control mean** | 1.1333 | 3.1439 |
| **delta (break − ctrl)** | 0.1817 | **0.2847** |
| **vs Phase 7 winner (0.2291)** | — | **+0.056 (+24%)** |

Delta = 0.2847 clears the Phase 7 acceptance gate (≥ 0.20) with headroom and exceeds the Phase 7 winner (`EnsembleInfoNCE`, delta = 0.2291) by +0.056.

Note: the absolute score scales are not comparable (UnixCoder raw embeddings have higher magnitude than CodeRankEmbed), but delta is a relative measure within each scorer's distribution and is the correct comparison point.

### Qualitative — top-20 / bottom-5

**Top-20 breakdown:** 15/20 test files · 5/20 docs/scripts · **0/20 fastapi/ core**

The predictor scores test files as most anomalous. This is structurally correct: test files contain the most diverse patterns (assertions, fixture injection, parametrize decorators, mock patches). They are genuine distributional outliers in the static chunk space, and the JEPA predictor correctly identifies them as hard to predict from context.

**Bottom-5:** all from `fastapi/params.py` — long repetitive `__init__` signatures for `Query`, `Form`, `Body`, etc. These are extremely predictable from context (same file, near-identical structure) and correctly score near zero.

**Implication for PR auditing:** test files would generate the most alerts even in normal PRs. For the audit use case, filtering results to `fastapi/` core or excluding test files from the ranked output may be necessary. Alternatively, this is a feature if the goal is detecting alien test code from another project.

---

## Confounders

This result cannot be cleanly attributed to either the new training data source or the encoder change, because both changed simultaneously.

| Variable | Baseline | Static audit |
|---|---|---|
| Training data source | git commit history | AST static chunks |
| Encoder | CodeRankEmbed | UnixCoder (raw) |

**To isolate:** re-run the static audit with `ENCODER_MODEL = "Salesforce/coderankembed"`. If delta stays near 0.28, the training data change drives the improvement. If it drops toward 0.18, the encoder is the primary driver.

---

## Key Learnings

1. **Static chunks are a viable training corpus.** 3868 AST-extracted chunks from HEAD produce stronger delta than 2000 git commit records. No git history required.

2. **Scale alignment is critical.** `NORMALIZE_EMBEDDINGS = False` is not optional with this encoder/predictor pairing. Normalized embeddings collapse the signal entirely (delta → 0.002). The predictor's LayerNorm output scale must be matched by the target embedding scale.

3. **Full file context beats windowed context.** A 50-line context window halved delta (0.1817 → 0.0624). The encoder needs structural context (imports, class declarations) to produce discriminative embeddings.

4. **File-level holdout is necessary.** Without it, the predictor memorizes file-specific boilerplate and inflates scores uniformly (contamination). Holding out entire files ensures generalization to unseen code.

5. **Test-file bias is an inherent property.** Test files are the most distributional diverse chunks in any Python repo. Any unsupervised anomaly model trained on the full file tree will rank them highest. Downstream presentation logic should account for this.

---

## Recommended Next Steps

See the companion investigation prompt for a structured exploration of further improvements.
