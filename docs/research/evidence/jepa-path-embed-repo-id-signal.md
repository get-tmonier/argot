# JEPA path embedding: the biggest number was repo identity

## Setup

Phase 3 sweep on structural side-features: the `path_embed` variant fits a
200-feature `TfidfVectorizer` on space-split file paths
(`packages/ai/openai/src/OpenAiLanguageModel.ts` →
`"packages ai openai src OpenAiLanguageModel ts"`) and concatenates the
resulting vector onto both context and hunk representations. All other
settings match the Phase 2 baseline. Three seeds per bucket across small
(3k), medium (7k), and large (20k).

Hypothesis: file paths carry cheap structural style signal — which
directories a repo uses (`src`, `packages`, `tests`, `internal`), filename
casing, and extension (`.ts` vs `.py`).

## Results

| bucket | shuffled       | cross-repo     | injected       |
|:-------|:---------------|:---------------|:---------------|
| small  | 0.505 ± 0.002  | 0.867 ± 0.006  | 0.712 ± 0.020  |
| medium | 0.634 ± 0.023  | 0.709 ± 0.038  | 0.662 ± 0.022  |
| large  | 0.610 ± 0.009  | 0.709 ± 0.006  | 0.615 ± 0.007  |

Cross-repo Δ vs baseline: small **+0.421** (baseline 0.446 → 0.867),
medium +0.315, large +0.164. The small-bucket number is the largest
cross-repo lift produced by any technique in the series.

Shuffled at large regresses (−0.027): the 200-dim path vocabulary occupies
model capacity that would otherwise go to intra-repo token-level style.
Injected at large is also flat (−0.016).

## Interpretation

The 0.867 at small is almost trivially explained. Effect stores all source
under `packages/*/src/*.ts`; pydantic stores tests under
`tests/test_*.py`. A 200-feature TF-IDF almost perfectly separates the
two repos by directory structure — *without reading a single line of
code*. Flagged as a **repo-identity** signal, not style: the model has
learned to fingerprint where a record came from, which is correct but
useless for a style linter asked "does this commit look consistent with
this repo?" This result, alongside the imports_scope finding, was the
era's first concrete hint that cross-repo AUC was not measuring what the
team thought it measured — a hint that hardened into the Phase 4 pivot.
