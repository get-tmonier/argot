# MLM surprise bakeoff: the zero-training baseline brought no signal

## Setup

Phase 12 Step 1 ran UniXCoder MLM surprise against the 51-fixture
v2 set at `file_only` context as a zero-training baseline — the
idea being that a pretrained masked language model ought to
assign higher reconstruction loss to non-idiomatic tokens without
any corpus-specific training. Three aggregations over hunk-level
token losses were measured: `mlm_surprise_mean`, `mlm_surprise_min`,
and `mlm_surprise_p05`. The implementation used joint-masking under MPS memory constraints (all hunk tokens masked in one forward pass), which changes the conditional from `P(tok | context, others_visible)` to `P(tok | context, all_hunk_masked)`.

## Results

| variant | overall_auc |
|:---|---:|
| `mlm_surprise_mean` | 0.4290 |
| `mlm_surprise_min` | 0.4129 |
| `mlm_surprise_p05` | 0.4129 |

Three aggregations mean/min/p05 all inverted (AUC 0.4129–0.4290) — the scorer systematically ranked controls as more anomalous than breaks. The CI against
`tfidf_anomaly` (paired bootstrap, n=1000) put the deltas cleanly
below zero for all three (e.g. `mlm_surprise_mean` delta CI
[−0.4452, −0.1000]). Per-category performance on the mean variant
was near-chance or below across the board (`background_tasks`
0.0000, `exception_handling` 0.1667, `async_blocking` 0.1667).
The per-token variant was left unmeasured under the same memory constraint.

## Interpretation

Joint-masking MLM was cleanly ruled out. Whether the inversion is
fundamental or an artifact of joint conditioning is unknown —
under joint masking the model cannot use intra-hunk structure to
disambiguate, which may suppress the signal MLM surprise is
supposed to carry. Either way, the bakeoff confirmed that a
pretrained LLM's out-of-the-box token-level surprise was not the
lever for this eval, and justified the investment already made in
JEPA (0.6532 > 0.43 on the same fixtures) before the Phase 12
re-benchmark of every other existing non-JEPA scorer.
