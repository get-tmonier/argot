# JEPA combined defaults: wins did not compound

## Setup

Phase 4 promoted three Phase-3 winners to defaults simultaneously and
re-ran the sizing benchmark:

- **char_ngrams** — `TfidfVectorizer(analyzer="char_wb", ngram_range=(3,5),
  max_features=5000)`
- **adaptive epochs** — `min(200, max(20, 1_400_000 // size))` — 200 at
  3k/7k, 70 at 20k
- **context_after** — always loaded and concatenated into `ctx_texts`

Per-bucket corpora (small 3k, medium 7k, large 20k), three seeds each.
Same protocol as the Phase 3 techniques. The question: do independent wins
compound when stacked?

## Results

| bucket | shuffled combined | cross combined | Δ vs char_ngrams (shuf / cross) |
|:-------|:------------------|:---------------|:--------------------------------|
| small  | 0.637 ± 0.008     | 0.394 ± 0.040  | +0.137 / **−0.325**             |
| medium | 0.616 ± 0.021     | 0.480 ± 0.101  | −0.039 / **−0.170**             |
| large  | 0.713 ± 0.013     | 0.622 ± 0.034  | +0.029 / −0.026                 |

Injected at large: 0.742 ± 0.013 (+0.111 vs baseline).

The combination beats the baseline on every shuffled and injected metric
at every size — but **it does not beat char_ngrams alone on cross-repo**.
Cross regresses by **−0.325** at small (vs char_ngrams 0.719) and −0.170
at medium. Shuffled AUC at large (0.713) is the best-in-era result on the
only metric not confounded by language detection — still well short of
usable.

## Interpretation

Two era-ending findings surface here. First, wins do not compound: adaptive
epochs and context_after pushed the model toward intra-repo coherence while
char_ngrams had been driving cross-repo gains, and the objectives competed
for capacity rather than adding up. Second, cross-repo and injected AUC were
largely measuring language detection — every bucket paired a TS repo with a
Py repo. Shuffled AUC (same repo, same language, tokens wrong order) was
the only honest metric left, and its 0.713 ceiling at 20k meant ~29%
failure on obvious violations. More tuning of the same harness would only
buy more confident wrong numbers; the pivot to honest eval followed.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/phases-1-6/09-combined.md`. Re-written here for
clarity, not copied.*
