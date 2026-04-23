# Simplex blend refuses to mix: token frequency carried everything

## Setup

Phase 12 Step 5 ran a convex simplex blend over top-3 scorers by individual AUC on the 51-fixture v2 set at `file_only` context: `tfidf_anomaly` (0.6968), `knn_cosine` (0.5306), and `ast_structural_zscore` (0.4887). The blend optimiser searched over
non-negative weights summing to 1 for the convex combination that
maximised overall AUC. If the three scorers had complementary
signal on different break categories, the optimiser should have
found a non-trivial mix; if token frequency was already carrying
everything, it should collapse to `tfidf_anomaly` alone.

## Results

Best α = `{tfidf: 1.0, knn: 0.0, ast_zscore: 0.0}`; blend AUC 0.6968 = tfidf alone. Per-category match was exact:

| category | blend | `tfidf_anomaly` |
|:---|---:|---:|
| async_blocking | 0.8333 | 0.8333 |
| background_tasks | 0.3750 | 0.3750 |
| dependency_injection | 0.6667 | 0.6667 |
| downstream_http | 1.0000 | 1.0000 |
| exception_handling | 0.5000 | 0.5000 |
| framework_swap | 0.6667 | 0.6667 |
| routing | 0.6667 | 0.6667 |
| serialization | 1.0000 | 1.0000 |
| validation | 0.7500 | 0.7500 |

The optimiser degenerated cleanly to the single best scorer.
`knn_cosine` would have been complementary on paper (strong on
`background_tasks` 0.75 and `framework_swap` 0.78 where tfidf is
weak), but its noise on the other categories outweighed the lift.
`ast_structural_zscore` raw scores are bimodal (0.0 for most
fixtures, large outliers for 3) — incompatible with a linear blend
after z-normalisation.

## Interpretation

The blend's refusal to mix was the most telling result of the era.
Three architecturally different scorers — token frequency,
embedding-space kNN, AST structural — offered no complementary
signal on the 51-fixture eval. If the next era was going to clear
the 0.80 victory gate, the additional signal would have to come
from somewhere structurally different than the hunk itself. Token
frequency had reached its ceiling.
