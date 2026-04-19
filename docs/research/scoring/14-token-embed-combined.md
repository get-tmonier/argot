# Phase 6 Workstream B — token_embed + context_after + adaptive_epochs

**Branch**: `research/token-embed-combined`  
**Status**: complete 2026-04-19  
**Baseline**: [`11-token-embeddings.md`](11-token-embeddings.md)

## Hypothesis

The Phase 4 regularisation techniques (`context_after`, `adaptive_epochs`) improved TF-IDF
across all metrics. Applied to `token_embed`, they should reduce the cross-repo collapse at
large scale: `context_after` doubles the context window (more signal), and `adaptive_epochs`
runs longer at small scale (more training) while capping at 200 to avoid overfitting.

## What changed

**`train.py::_train_token_embed`**: added `context_after: bool` parameter. When True,
`context_after` tokens are concatenated to `context_before` before encoding.

**`corpus.py`**: added `adaptive_epochs(size)` function (`min(200, max(20, 1_400_000 // size))`),
`context_after` parameter wired through `run_benchmark` / `_benchmark_one` / `train_model`.
Added `--context-after` flag and `--epochs 0` sentinel (meaning: use adaptive formula).

**Effective epochs**: 200 at small (3k) and medium (7k), 70 at large (20k).

## Results

Baseline is Phase 5 token_embed with fixed 20 epochs, no context_after.

### Shuffled AUC (primary metric)

| bucket | token_embed (baseline) | token_embed_combined | Δ        |
|:-------|:-----------------------|:---------------------|:---------|
| small  | 0.639 ± 0.012          | 0.701 ± 0.023        | +0.062   |
| medium | 0.701 ± 0.014          | 0.582 ± 0.026        | **-0.119** |
| large  | 0.697 ± 0.019          | 0.713 ± 0.005        | +0.016   |

### Cross-repo AUC

| bucket | token_embed (baseline) | token_embed_combined | Δ          |
|:-------|:-----------------------|:---------------------|:-----------|
| small  | 0.445 ± 0.090          | 0.417 ± 0.129        | -0.028     |
| medium | 0.665 ± 0.026          | 0.543 ± 0.047        | **-0.122** |
| large  | 0.457 ± 0.076          | 0.317 ± 0.008        | **-0.140** |

### Injected AUC

| bucket | token_embed (baseline) | token_embed_combined | Δ          |
|:-------|:-----------------------|:---------------------|:-----------|
| small  | 0.776 ± 0.070          | 0.641 ± 0.024        | **-0.135** |
| medium | 0.761 ± 0.024          | 0.570 ± 0.020        | **-0.191** |
| large  | 0.687 ± 0.005          | 0.691 ± 0.019        | +0.004     |

### Raw per-run data

| size  | seed | epochs | shuffled | cross | injected |
|------:|-----:|-------:|---------:|------:|---------:|
| 3000  | 0    | 200    | 0.689    | 0.288 | 0.611    |
| 3000  | 1    | 200    | 0.680    | 0.593 | 0.670    |
| 3000  | 2    | 200    | 0.733    | 0.369 | 0.643    |
| 7000  | 0    | 200    | 0.612    | 0.527 | 0.576    |
| 7000  | 1    | 200    | 0.585    | 0.607 | 0.590    |
| 7000  | 2    | 200    | 0.549    | 0.495 | 0.543    |
| 20000 | 0    | 70     | 0.718    | 0.314 | 0.677    |
| 20000 | 1    | 70     | 0.715    | 0.327 | 0.717    |
| 20000 | 2    | 70     | 0.706    | 0.309 | 0.678    |

## Interpretation

**Medium regression is severe.** Shuffled AUC drops from 0.701 to 0.582 — a -0.119 collapse.
Adaptive epochs gives 200 at 7k (vs 20 fixed), 10× more training. For a dense embedding table,
this is catastrophic overfitting: the model memorises training-split token distributions and
fails to generalise to held-out. TF-IDF is immune to this because bag-of-words features do not
overfit the same way.

**Cross-repo at large collapses further.** 0.317 vs 0.457 for plain token_embed — the
combination of `context_after` (more tokens, more repo-specific signal) and 70 epochs (vs 20)
makes the embedding table even more specialised to in-repo subword patterns. The model
memorises more aggressively, not less.

**Context_after truncation effect.** Concatenating `context_after` fills the 256-token window
faster, truncating more of the actual hunk context. At medium and large where contexts are
longer, this may remove useful positional signal.

**Small shuffled AUC improves (+0.062)** because 200 epochs genuinely helps learn representations
from limited data. But cross-repo variance is extreme (0.288–0.593), suggesting random
initialisation luck dominates.

## Success criterion evaluation

- **Cross-repo AUC at large ≥ 0.550**: ✗ **FAILED** (0.317 ± 0.008 — catastrophic regression)
- **Shuffled AUC at large ≥ 0.680**: ✓ **MET** (0.713 ± 0.005)

## Verdict

**This combination is harmful for token_embed.** The Phase 4 techniques that regularise TF-IDF
actually destabilise the dense encoder:

- `adaptive_epochs` at medium (200 epochs) → severe overfitting
- `context_after` → more repo-specific signal in the context window, worsens cross-repo
- Together they push cross-repo at large to 0.317 — the worst result in the entire research

**Key insight**: the regularisation intuitions from TF-IDF do not transfer to dense encoders.
TF-IDF is a fixed, stateless transform — more context and more epochs cannot overfit it.
Token embeddings are learned weights — more training at medium/large leads to memorisation,
not generalisation. The correct regularisation for dense encoders is architectural (dropout,
weight decay) or data-driven (larger corpus, same-language pairs), not epoch scaling.

**Do not merge.** Plain `token_embed` with 20 fixed epochs is strictly better than this
combination at medium and large scale.
