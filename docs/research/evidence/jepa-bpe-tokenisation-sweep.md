# JEPA BPE tokenisation sweep: partial recovery, not a fix

## Setup

Phase 6 replaced the raw token vocabulary with byte-level BPE subwords
(HuggingFace `tokenizers`, `vocab_size=8000`, `min_frequency=2`). The
encoder stack is otherwise identical to `token_embed`: `nn.Embedding`
(8000 × 128) → masked mean pool over seq_len=256 → linear to 192. Params
and artifact size unchanged (~1.05M, ~4 MB). Phase 6 discipline:
`context_after` and `adaptive_epochs` are **off**; only tokenisation
changes.

Hypothesis: token_embed's cross-repo collapse at large came from OOV —
raw token texts (`myVariableName`) are repo-specific, so on a foreign
repo most tokens map to `<unk>`. BPE splits into shared subwords
(`my`, `Variable`, `Name`), which should reduce OOV and let the
embedding table generalise.

## Results

| bucket | shuffled       | cross-repo     | injected       |
|:-------|:---------------|:---------------|:---------------|
| small  | 0.629 ± 0.016  | 0.710 ± 0.005  | 0.779 ± 0.018  |
| medium | 0.696 ± 0.016  | 0.684 ± 0.024  | 0.720 ± 0.054  |
| large  | **0.704 ± 0.017** | 0.514 ± 0.054 | 0.698 ± 0.026 |

Shuffled at large: **0.704 ± 0.017**, a **+0.007** gain vs token_embed
(0.697) — BPE preserves the primary metric and slightly improves it at
large.

Cross-repo at small jumps from 0.445 (token_embed, near-random) to 0.710 —
essentially matching char_ngrams (0.719). At medium, 0.684 edges both
token_embed (0.665) and char_ngrams (0.650). At large, cross-repo recovers
from 0.457 to 0.514 (+0.057), about 27% of the gap back toward the TF-IDF
baseline (0.544) — with one seed regressing to 0.443.

## Interpretation

BPE nearly eliminates the OOV collapse at small and medium — a clean fix
for the worst of the token_embed failure mode. At large, the collapse is
reduced but not gone: with 20k records the embedding table has enough
capacity to memorise subword distributions that are still partly
repo-specific (library-specific function-name fragments). BPE was the
last sweep of the JEPA era; its partial result — primary metric held,
cross-repo still stuck — confirmed that tuning alone would not clear the
plateau, and the pivot to honest eval followed immediately.
