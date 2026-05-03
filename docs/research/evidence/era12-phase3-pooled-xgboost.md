# Era 12 — Phase 3: Pooled XGBoost training (KILL-SWITCH #2)

**Date**: 2026-05-03
**Branch**: `docs/era-10-root-readme` (training/analysis only)
**Inputs**: `engine/.era12-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl`
**Code**: `engine/argot/ml/train.py`
**Artifacts**: `.era12-features/{pooled_setA.joblib, pooled_setB.joblib, feature_list.json}`
**Method**: `xgboost.XGBClassifier`, pre-registered hyperparameters, 5-fold stratified CV.

---

## TL;DR

| Set | Total | Breaks | Controls | 5-fold CV AUC | Gate | Result |
|-----|------:|-------:|---------:|--------------:|-----:|-------:|
| A (full pooled) | 1,915 | 115 | 1,800 | **0.9991 ± 0.0009** | > 0.85 | PASS |
| B (Stage-4 regime) | 1,453 | 12 | 1,441 | **0.9998 ± 0.0005** | > 0.70 | PASS |

**KILL-SWITCH verdict: PASS — era 12 advances to Phase 4 (leave-one-corpus-out).**

Caveat: Set B has only 12 positive rows (5 of which are the faker-js residuals). The CV AUC is precise but the *generalisation* signal is weak — Phase 4 is the actual stress test. The 5/5 catch on residuals at faker-js FP ≤ 0.9% is encouraging but not proof of cross-corpus transfer.

---

## Task 1 — Set sizes

Set A is the full pool. Set B is the Stage-4 operating regime: rows where the production scorer reported `scorer_reason ∈ {"none", null}` (i.e. stages 1-3 returned no flag).

```
Set A: total=1915  breaks=115   controls=1800
Set B: total=1453  breaks= 12   controls=1441   (faker-js residuals in B: 5)
```

Per-corpus break breakdown by `scorer_reason` (from raw inputs):

| corpus    | bpe | call_receiver | import | none |
|-----------|----:|--------------:|-------:|-----:|
| fastapi   |  22 |             6 |      1 |    3 |
| rich      |   6 |             2 |      8 |    0 |
| faker     |   8 |             2 |      5 |    1 |
| hono      |   9 |             4 |      2 |    2 |
| ink       |  14 |             1 |      1 |    1 |
| faker-js  |   3 |             7 |      2 |    5 |

The 5 faker-js residuals are the well-known `error_flip_2/3` + `runtime_fetch_1/2/3` set. Set B keeps all of them plus 7 other "missed by stages 1-3" fixtures across the 5 other corpora.

Set B controls (1,441) are far from "trivial" — 740/1,441 (51%) have nonzero `n_distinct_callees`, and 730/1,441 (51%) have nonzero `hunk_file_callee_jaccard`. faker-js is the sparsest corpus here (217 controls, only 34 with nonzero jaccard).

---

## Task 2 — Feature engineering

**Dropped (per Phase 2 redundancy findings):**
- `n_distinct_callees` — bit-for-bit identical to `hunk_callee_bag_size`.
- `stage1_flagged` — r=0.963 with `import_score`.

**AST top-5 (computed from pooled `ast_node_type_counts` summed over Set A):**

`identifier`, `string`, `property_identifier`, `string_fragment`, `pair`

These are TS-leaning (Python's analogue is just `identifier`). `string_fragment` and `pair` are mostly TS template-literal/object-literal features, which is why they survive the top-5 cut even though Python rows don't contribute.

**Engineered features added:**
- `n_total_ast_nodes` (sum of all named-node counts)
- 5 per-type counts: `ast_identifier`, `ast_string`, `ast_property_identifier`, `ast_string_fragment`, `ast_pair`

**Final numeric feature list (25 features, ordered):**

```
adjusted_bpe                  cluster_jaccard_to_centroid       hunk_callees_in_file_fraction
bpe_score                     n_unattested_callees              hunk_file_callee_jaccard
import_score                  n_attested_root_only              n_returns
cluster_id                    n_cluster_absent_callees          n_throws
                              hunk_callee_bag_size              n_awaits
                              file_callee_bag_size              max_nesting_depth
                                                                n_distinct_identifiers
                                                                parse_fragment_flag
                                                                stage2_flagged
n_total_ast_nodes
ast_identifier  ast_string  ast_property_identifier  ast_string_fragment  ast_pair
```

Booleans coerced to 0/1. Categorical `cluster_id` left as integer (XGBoost handles it as ordinal — fine for tree splits).

Manifest saved to `.era12-features/feature_list.json`.

---

## Task 3 — Pooled XGBoost training

Pre-registered hyperparameters (no tuning):
```
n_estimators=100, max_depth=4, learning_rate=0.1,
objective="binary:logistic", random_state=0, eval_metric="auc", tree_method="hist"
```

5-fold stratified `cross_val_score(scoring="roc_auc")`:

| Set | Mean AUC | Std    | Per-fold |
|-----|---------:|-------:|---------|
| A   | 0.9991   | 0.0009 | 0.9975, 0.9990, 0.9999, 0.9990, 0.9999 |
| B   | 0.9998   | 0.0005 | 1.0000, 0.9988, 1.0000, 1.0000, 1.0000 |

Per-corpus AUC computed from out-of-fold predictions on the pooled set (NOT LOO):

| corpus    | Set A n / pos / AUC      | Set B n / pos / AUC       |
|-----------|---------------------------|---------------------------|
| fastapi   | 332 / 32 / **0.9992**    | 296 /  3 / **1.0000**    |
| rich      | 316 / 16 / **1.0000**    | 186 /  0 / n/a            |
| faker     | 316 / 16 / **0.9994**    | 192 /  1 / **0.9948**    |
| hono      | 317 / 17 / **1.0000**    | 264 /  2 / **1.0000**    |
| ink       | 317 / 17 / **1.0000**    | 293 /  1 / **1.0000**    |
| faker-js  | 317 / 17 / **1.0000**    | 222 /  5 / **1.0000**    |

Consistency is high across corpora. `rich` has zero positives in Set B (every rich fixture was caught by stages 1-3) so its AUC is undefined — that's fine, but it means LOO with rich as held-out corpus on Set B will be impossible.

---

## Task 4 — KILL-SWITCH evaluation

Pre-registered Phase 3 gates:

| Gate                          | Threshold | Observed | Pass? |
|-------------------------------|----------:|---------:|------:|
| Set A 5-fold CV AUC           |    > 0.85 |   0.9991 |   YES |
| Set B 5-fold CV AUC           |    > 0.70 |   0.9998 |   YES |

**Verdict: PASS.** Era 12 advances to Phase 4 (LOO).

**Caveat — please read before celebrating:** Set B has 12 positives total. A pooled CV with that few positives reliably tests "can the model fit the training data" — it does not test cross-corpus generalisation. Phase 4 (LOO) will pull all positives from one corpus out of training; with `rich` and `ink` having only 0–1 positives in Set B, several LOO folds will be measurement-degenerate. Plan for this in Phase 4 design.

---

## Task 5 — Feature importance (gain)

**Set A — top 10:**

| Rank | Feature                          | Gain   |
|-----:|----------------------------------|-------:|
|    1 | adjusted_bpe                     | 32.00  |
|    2 | cluster_jaccard_to_centroid      | 29.93  |
|    3 | hunk_callee_bag_size             | 17.65  |
|    4 | file_callee_bag_size             | 13.99  |
|    5 | ast_string                       |  3.72  |
|    6 | max_nesting_depth                |  3.66  |
|    7 | n_distinct_identifiers           |  3.58  |
|    8 | cluster_id                       |  3.07  |
|    9 | bpe_score                        |  1.24  |
|   10 | n_attested_root_only             |  1.17  |

**Set B — top 10:**

| Rank | Feature                          | Gain   |
|-----:|----------------------------------|-------:|
|    1 | ast_string_fragment              | 17.50  |
|    2 | n_total_ast_nodes                | 12.85  |
|    3 | cluster_jaccard_to_centroid      |  6.79  |
|    4 | cluster_id                       |  4.02  |
|    5 | adjusted_bpe                     |  3.99  |
|    6 | file_callee_bag_size             |  3.74  |
|    7 | hunk_file_callee_jaccard         |  2.79  |
|    8 | n_distinct_identifiers           |  2.12  |
|    9 | bpe_score                        |  1.97  |
|   10 | hunk_callee_bag_size             |  1.36  |

**Overlap:** Set A and Set B share 6/10 top features (cluster_jaccard_to_centroid, adjusted_bpe, file_callee_bag_size, n_distinct_identifiers, bpe_score, hunk_callee_bag_size). The biggest divergence is at rank 1: Set A leans heavily on `adjusted_bpe`, while Set B *can't* — many Set B controls already have a high `adjusted_bpe` (they're in the regime where stages 1-3 declined to flag despite a non-trivial BPE), so the model has to look elsewhere.

**Surprises:**
- `ast_string_fragment` rises to #1 for Set B. This is a TS-only feature (template literals like `` `Hello ${name}` ``). The 5 faker-js residuals all come from .ts fixtures that involve `runtime_fetch` (URLs as template strings) and `error_flip` (likely throwing strings) — so the model is plausibly latching onto a real TS-string-construction signal. But at n=12 positives it might also be coincidence; treat as Phase-4 follow-up.
- `n_total_ast_nodes` at #2 for Set B suggests hunk size is itself discriminative once stages 1-3 are stripped.
- `hunk_file_callee_jaccard` shows up at #7 for Set B, consistent with the Phase 2 finding that this feature is the strongest single residual signal (1.0 for all 5 faker-js residuals vs control median 0).
- Tree models *do* exploit interactions: `ast_string_fragment` was AUC ~0.5 in Phase 2 (not in the top-21) but becomes Set B's most important splitter. Expected from depth-4 trees, exactly what Phase 2 hypothesised.

---

## Task 6 — Residual faker-js fixtures (Phase-5 soft preview)

5-fold OOF predictions from the Set B model, applied to the 5 faker-js residual fixtures:

| Fixture                       | OOF p   | Controls above (of 217 faker-js) | FP @ thresh |
|-------------------------------|--------:|---------------------------------:|------------:|
| faker_js_runtime_fetch_1      | 0.9626  |                            0/217 |      0.00%  |
| faker_js_runtime_fetch_2      | 0.9400  |                            0/217 |      0.00%  |
| faker_js_error_flip_3         | 0.9322  |                            0/217 |      0.00%  |
| faker_js_runtime_fetch_3      | 0.9314  |                            0/217 |      0.00%  |
| faker_js_error_flip_2         | 0.9298  |                            0/217 |      0.00%  |

Every residual scores higher than every faker-js control. At threshold 0.0093 (the Phase-5 reference, faker-js FP ≤ 0.9%, actual realised FP **0.46%**) the model catches **5/5** residuals.

This is much better than the asked-for ≥2/5 — but it's an *in-sample* (no LOO) cross-validated prediction. The OOF training folds do see other corpora's positives, so the model learns "fixtures look different from controls" with full information about the residual signal in this corpus. Phase 4 will tell us if that transfers when faker-js is held out entirely.

**Sanity-check why it works:** the Set B faker-js controls are sparse on the residual-signal features (only 34/217 have nonzero `hunk_file_callee_jaccard`), and the residuals all hit jaccard=1.0 + nonzero callees. A depth-4 tree can carve out that joint region cleanly.

---

## Phase 4 design implications

1. **LOO folds with sparse positives need protection.** rich has 0 Set B positives, ink has 1, faker has 1. AUC on those LOO folds will be undefined or single-point. Consider stratifying LOO by reporting AUC on held-out corpora with ≥3 positives only, or fall back to per-fixture rank statistics.
2. **Re-evaluate `ast_string_fragment` importance under LOO.** If it stays at the top, the model is genuinely using TS-template-string structure. If it crashes when faker-js is held out, it was overfitting to the residual fixtures.
3. **Set A LOO is the safer kill-switch.** Set A has ≥16 positives per corpus, every fold will give a real AUC.
4. **Don't tune yet.** The pre-registered defaults already saturate. Tuning on Set B without LOO would just memorise 12 positives.

---

## Outputs

- This memo: `docs/research/evidence/era12-phase3-pooled-xgboost.md`
- Trained models: `.era12-features/pooled_setA.joblib`, `.era12-features/pooled_setB.joblib`
- Feature manifest: `.era12-features/feature_list.json`
- Training script: `engine/argot/ml/train.py` (run via `uv run python -m argot.ml.train` from the engine venv; requires `xgboost` + system `libomp` — added to the local venv only, not the project pyproject)
- Optional CLI (`argot-train-stage4`) **not implemented** — kept the surface minimal to leave Phase 6 productionisation choices open for the orchestrator.

## Notes on environment

`xgboost` and `libomp` were installed locally to satisfy Phase-3 training but **were not added to `engine/pyproject.toml`**. If Phase 6 productionises this scorer, the dependency needs to be added there (and CI's setup will need `brew install libomp` on macOS runners or `libgomp` on Linux).
