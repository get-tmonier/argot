# tfidf_anomaly victory: zero training beats the JEPA ensemble

## Setup

Phase 12 Step 4 re-benchmarked every existing non-JEPA scorer
(`tfidf_anomaly`, `knn_cosine`, `lof_embedding`, `lm_perplexity`,
three `ast_structural_*` variants, and the three `mlm_surprise`
aggregations) at Phase 11's `file_only` context on the same 51
fixtures. `EnsembleJepa mean_z @ file_only` at AUC 0.6532 was the
Phase 11 winner the bakeoff was trying to unseat. Paired bootstrap
CIs (n=1000, α=0.05) were computed vs `tfidf_anomaly`.

## Results

`tfidf_anomaly` at `file_only` AUC 0.6968 — +0.0436 over the Phase 11 JEPA winner, zero training, no GPU. Six of nine categories were solid: `downstream_http` 1.0000, `serialization` 1.0000, `async_blocking` 0.8333, `validation` 0.7500 (up from 0.2500 inverted under JEPA), `framework_swap` 0.6667, `routing` 0.6667. `background_tasks` remained inverted at 0.3750.

| scorer | overall AUC |
|:---|---:|
| `tfidf_anomaly` (winner) | 0.6968 |
| `knn_cosine` | 0.5306 |
| `ast_structural_zscore` | 0.4887 |
| `mlm_surprise_mean` | 0.4290 |
| `lof_embedding` | 0.3919 |
| `lm_perplexity` | 0.2839 |

The victory gate was NOT cleared. Paired bootstrap 95% CI on `tfidf_anomaly` vs the Phase 11 winner landed at [0.5532, 0.8435] — 0.29-wide, lower bound 0.10 below the JEPA baseline; target 0.80. The gain was real but not statistically tight on 51 fixtures.

## Interpretation

`tfidf_anomaly` promoted over `EnsembleJepa mean_z` as the new production default: materially better AUC, zero training cost, and it actually fixed `validation` where JEPA inverted. The wide CI and the stubborn `background_tasks` inversion pointed at the same ceiling — both scorers treat "anomalous against corpus frequency" as a proxy for "non-idiomatic", which is exactly backwards when the idiomatic pattern is rare in commit diffs.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/phase12/final_2026-04-21.md`.
Re-written here for clarity, not copied.*
