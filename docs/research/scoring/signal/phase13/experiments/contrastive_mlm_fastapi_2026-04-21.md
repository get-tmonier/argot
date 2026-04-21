# Phase 13 — Contrastive-MLM Experiment (FastAPI, 2026-04-21)

## Summary

| scorer | corpus | approach | AUC |
|---|---|---|---|
| contrastive_tfidf (word) | FastAPI | marginal token freq | 0.9847 |
| bpe_contrastive_tfidf | FastAPI | marginal BPE freq | 1.0000 |
| contrastive_jepa | FastAPI | sentence embedding | 0.5532 |
| **contrastive_mlm** | **FastAPI** | **conditional MLM log-ratio** | **0.4645** |

## Gate

**FAILED** — AUC 0.4645 < 0.85. Below-random (inverted). Do not proceed to click.

## Diagnosis

### What we observed

AUC 0.4645 is not just random (0.5) — it is actively inverted. Breaks score *lower* than
controls on the `max(log P_B − log P_A)` aggregation. This rules out "no signal" and points
to a specific failure mode.

### Root cause: insufficient fine-tuning creates wrong-direction divergence

The LoRA adapter was trained for 1 epoch on 20 FastAPI control files (~200 lines each).
Against a 125M-parameter model pre-trained on millions of Python files, this is a rounding
error — the adapter barely moves from its random initialization.

However, "barely" is not "zero". What little the adapter learned is general Python fluency,
not FastAPI-specific patterns. With even a small improvement in predicting common Python
tokens, model A (adapters ON) becomes marginally *better* than model B (vanilla CodeBERT)
at predicting **all** Python — including the foreign tokens in break fixtures. This gives:

```
log P_B(t) − log P_A(t) < 0   for most tokens in most fixtures
```

The `max` aggregation then picks the *least negative* token per fixture. Controls (pure
FastAPI idioms that model A slightly favors) end up with higher max scores than breaks
(foreign tokens that model A also slightly favors, due to general Python learning). Hence
the inversion.

### Why more training would not fix this

Even with 10× more epochs, the FastAPI control corpus is too small and too homogeneous to
teach model A a *FastAPI-specific* distribution distinct from general Python. The contrastive
signal requires:

- Model A: deeply specialized on the target repo's idioms
- Model B: a broad general Python baseline

With 20 short files, model A cannot become specialized enough. The gap between A and B
remains dominated by initialization noise rather than learned repo-specific patterns.

### Why contrastive-JEPA had a related failure (AUC 0.5532)

Contrastive-JEPA also failed to beat the tfidf baselines. Both MLM and JEPA share the same
structural weakness: the "repo-specific model A" cannot be meaningfully trained on a corpus
this small. The contrastive signal requires a large, distinctive training set — something
the FastAPI control split (20 files) does not provide.

## What this means for Phase 13

Token-frequency contrast (word-tfidf, BPE-tfidf) succeeds on FastAPI precisely because it
does **not** require training — the reference is the CPython stdlib (large, stable). The
contrastive signal comes from the marginal distribution gap between stdlib and FastAPI, which
is large and well-defined.

Conditional-probability approaches (MLM, JEPA) require a trained model A that genuinely
captures the target repo's joint token distribution. That requires far more data than any
single repo's control split can provide.

## Recommendation

Abandon the MLM fine-tuning direction. The click ceiling (AUC 0.60–0.70 for BPE-tfidf) is
a context problem, not a training-data problem. The right fix is a **context-aware scoring
function that requires no training**:

- **Next candidate**: tree-sitter structural diff — score hunks by how much their AST node
  distribution diverges from the repo's AST profile (no model training, purely structural).
- **Alternative**: n-gram LM perplexity against a large pretrained model (e.g., CodeLlama)
  used zero-shot — no fine-tuning, full conditional distributions, no OOM risk.
