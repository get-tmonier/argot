# Era 14 — Phase 2: Per-feature discrimination AUC (KILL-SWITCH)

**Date**: 2026-05-03
**Branch**: `docs/era-10-root-readme` (analysis-only)
**Inputs**: `engine/.era14-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl`
**Total rows**: 1,915 = 115 fixtures + 1,800 controls (300 per corpus)
**Method**: `sklearn.metrics.roc_auc_score` on raw feature value (no model fit)

---

## Sanity checks (Task 5) — PASS

| corpus    | total | breaks | expected | controls | other |
|-----------|------:|-------:|---------:|---------:|------:|
| fastapi   |   332 |     32 |       32 |      300 |     0 |
| rich      |   316 |     16 |       16 |      300 |     0 |
| faker     |   316 |     16 |       16 |      300 |     0 |
| hono      |   317 |     17 |       17 |      300 |     0 |
| ink       |   317 |     17 |       17 |      300 |     0 |
| faker-js  |   317 |     17 |       17 |      300 |     0 |

Fixture counts match catalog. Controls all `is_break=false`. Booleans (`parse_fragment_flag`, `stage1_flagged`, `stage2_flagged`) cast to {0.0, 1.0} for AUC. Numeric values look plausible (e.g. faker-js residual `bpe_score` 1.97–4.55, `n_distinct_callees` ∈ {1, 2}).

Skipped: `cluster_id` (categorical), `ast_node_type_counts` (dict), `scorer_reason` + `cluster_assignment_method` (strings).

---

## Task 1 — Per-feature AUC (sorted by pooled, desc)

Direction column: `+` = higher value → more anomalous (raw AUC); `-` = lower value → more anomalous (negate before AUC).

| Feature                          | Dir | Pooled | fastapi | rich  | faker | hono  | ink   | faker-js |
|----------------------------------|:---:|-------:|--------:|------:|------:|------:|------:|---------:|
| adjusted_bpe                     |  +  |  0.932 |  0.946  | 0.997 | 0.931 | 0.916 | 0.997 |   0.785  |
| cluster_jaccard_to_centroid      |  -  |  0.912 |  0.993  | 1.000 | 0.934 | 1.000 | 0.992 |   0.570  |
| n_unattested_callees             |  +  |  0.854 |  0.977  | 0.860 | 0.955 | 0.843 | 0.751 |   0.682  |
| bpe_score                        |  +  |  0.833 |  0.908  | 0.929 | 0.854 | 0.679 | 0.953 |   0.574  |
| hunk_callee_bag_size             |  +  |  0.788 |  0.908  | 0.860 | 0.878 | 0.741 | 0.521 |   0.867  |
| n_distinct_callees               |  +  |  0.788 |  0.908  | 0.860 | 0.878 | 0.741 | 0.521 |   0.867  |
| n_cluster_absent_callees         |  +  |  0.776 |  0.906  | 0.999 | 0.735 | 0.761 | 0.545 |   0.614  |
| stage2_flagged                   |  +  |  0.765 |  0.926  | 0.560 | 0.631 | 0.819 | 0.928 |   0.656  |
| max_nesting_depth                |  +  |  0.749 |  0.823  | 0.813 | 0.745 | 0.485 | 0.665 |   0.954  |
| n_attested_root_only             |  +  |  0.707 |  0.854  | 0.579 | 0.796 | 0.729 | 0.640 |   0.540  |
| n_returns                        |  +  |  0.704 |  0.918  | 0.481 | 0.744 | 0.449 | 0.553 |   0.911  |
| n_distinct_identifiers           |  +  |  0.617 |  0.840  | 0.624 | 0.768 | 0.413 | 0.301 |   0.693  |
| n_throws                         |  +  |  0.594 |  0.768  | 0.490 | 0.472 | 0.497 | 0.576 |   0.577  |
| import_score                     |  +  |  0.583 |  0.516  | 0.750 | 0.656 | 0.559 | 0.529 |   0.559  |
| stage1_flagged                   |  +  |  0.583 |  0.516  | 0.750 | 0.656 | 0.559 | 0.529 |   0.559  |
| n_awaits                         |  +  |  0.520 |  0.532  | 0.500 | 0.531 | 0.432 | 0.447 |   0.647  |
| file_callee_bag_size             |  +  |  0.337 |  0.413  | 0.050 | 0.552 | 0.301 | 0.113 |   0.692  |
| parse_fragment_flag              |  +  |  0.316 |  0.375  | 0.378 | 0.367 | 0.238 | 0.207 |   0.277  |
| hunk_callees_in_file_fraction    |  -  |  0.245 |  0.310  | 0.163 | 0.292 | 0.238 | 0.461 |   0.038  |
| hunk_file_callee_jaccard         |  -  |  0.114 |  0.136  | 0.021 | 0.182 | 0.102 | 0.312 |   0.007  |

**Notes on directionality.** Two `-` features score AUC ≪ 0.5 even after negation — i.e. anomalous hunks have *higher* `hunk_file_callee_jaccard` / `hunk_callees_in_file_fraction` than controls (consistent with pre-flagged stage-2 receivers in the call_receiver scorer family). Treating them as `+` would push these to ~0.755 / ~0.886 pooled. The pre-registered hypothesis in the task brief was wrong about direction; the model layer can flip it freely. `file_callee_bag_size` and `parse_fragment_flag` are similarly inverted (controls > breaks).

---

## Task 2 — KILL-SWITCH verdict: **PASS**

- **Best pooled AUC**: `adjusted_bpe` = **0.932** (gate is > 0.55 — cleared by 38× the margin).
- **5 features clear the stricter "AUC > 0.55 on every individual corpus" bar**:
  - `adjusted_bpe` (worst per-corpus 0.785, faker-js)
  - `cluster_jaccard_to_centroid` (worst 0.570, faker-js)
  - `n_unattested_callees` (worst 0.682, faker-js)
  - `bpe_score` (worst 0.574, faker-js)
  - `stage2_flagged` (worst 0.560, rich)
- Era 14 advances to **Phase 3** (XGBoost training).

The pattern is clear: every numerator-strong feature is weakest on **faker-js**, which matches era-11/era-12 evidence that faker-js fixtures (especially the 5 residuals) live in distribution territory close to controls.

---

## Task 3 — Residual faker-js fixture inspection

faker-js control medians (n=300):

| feature                         | median  |   q25   |   q75   |
|---------------------------------|--------:|--------:|--------:|
| bpe_score                       |  3.8224 | -0.1610 |  4.8607 |
| n_cluster_absent_callees        |  0.0000 |  0.0000 |  0.0000 |
| hunk_file_callee_jaccard        |  0.0000 |  0.0000 |  0.0000 |
| hunk_callees_in_file_fraction   |  0.0000 |  0.0000 |  0.0000 |
| n_distinct_callees              |  0.0000 |  0.0000 |  1.0000 |

The 5 residuals:

| fixture                       | bpe_score | n_clust_abs | hunk_jac | hunk_in_file | n_callees | scorer_reason |
|-------------------------------|----------:|------------:|---------:|-------------:|----------:|:--------------|
| faker_js_error_flip_2         |    4.5463 |      0.0000 |   1.0000 |       1.0000 |    1.0000 | none          |
| faker_js_error_flip_3         |    4.0527 |      0.0000 |   1.0000 |       1.0000 |    1.0000 | none          |
| faker_js_runtime_fetch_1      |    3.5478 |      0.0000 |   1.0000 |       1.0000 |    2.0000 | none          |
| faker_js_runtime_fetch_2      |    2.4487 |      0.0000 |   1.0000 |       1.0000 |    1.0000 | none          |
| faker_js_runtime_fetch_3      |    1.9712 |      0.0000 |   1.0000 |       1.0000 |    2.0000 | none          |

**Verdict: DISTINGUISHABLE — ML has a chance.** All 5 residuals sit outside the control IQR on **>=2 of 5** features:

- `hunk_file_callee_jaccard = 1.000` for every residual vs control median 0 (control p75 = 0). 100% receiver overlap with file is rare among controls — only ~25% of controls are nonzero here.
- `hunk_callees_in_file_fraction = 1.000` similarly anomalous.
- `bpe_score` mostly within control range (1.97–4.55 vs control IQR -0.16 to 4.86) — weak signal.
- `n_cluster_absent_callees = 0` for all — useless on residuals.

The opposite-direction signal on `hunk_file_callee_jaccard`/`hunk_callees_in_file_fraction` (residuals look *more* file-like than controls, not less) explains why the production scorer with `parse_fragment` gating misses them: it was only configured to fire on the *low* end. An ML model that learns the joint pattern (high jaccard + nonzero `n_distinct_callees` + moderate bpe) has discriminative material to work with.

`scorer_reason = "none"` for all 5 confirms the production stage-1+stage-2 cascade does not flag these — Phase-3 has to learn this from raw features.

---

## Task 4 — Top-5 feature redundancy (Pearson on pooled n=1915)

|                              | adjusted_bpe | cluster_jac | n_unatt | bpe_score | hunk_bag |
|------------------------------|-------------:|------------:|--------:|----------:|---------:|
| adjusted_bpe                 |        1.000 |      -0.526 |   0.453 |     0.718 |    0.276 |
| cluster_jaccard_to_centroid  |       -0.526 |       1.000 |  -0.254 |    -0.421 |    0.003 |
| n_unattested_callees         |        0.453 |      -0.254 |   1.000 |     0.084 |    0.315 |
| bpe_score                    |        0.718 |      -0.421 |   0.084 |     1.000 |    0.231 |
| hunk_callee_bag_size         |        0.276 |       0.003 |   0.315 |     0.231 |    1.000 |

**No pair exceeds 0.95 in the top-5.** Highest is `adjusted_bpe ↔ bpe_score` at 0.718 (expected — `adjusted_bpe = bpe_score + density_head_bonus`). All five carry meaningfully independent signal.

**Caveat — exact duplicates outside top-5:**
- `n_distinct_callees` ≡ `hunk_callee_bag_size` — bit-for-bit identical across all 1,915 rows. Drop one before XGBoost.
- `stage1_flagged` ↔ `import_score`: r = 0.963. `stage1_flagged` is a binarization of `import_score` (≥1 → 1). Treat as one signal.

These two redundancies were not flagged by the strict top-5 gate but matter for Phase-3 feature-importance interpretability.

---

## Phase 3 design implications

1. **Drop `n_distinct_callees`** (perfect dup of `hunk_callee_bag_size`).
2. **Drop one of `stage1_flagged` / `import_score`** (r=0.963).
3. **Allow XGBoost to learn its own monotonicity**: `hunk_file_callee_jaccard` and `hunk_callees_in_file_fraction` flip sign on faker-js residuals — do NOT pre-negate.
4. **Stratify CV by corpus**: faker-js is the hardest target; per-corpus AUC ranges from 0.57 to 0.95 across the top-5 features. Random splits will overstate generalization.
5. **Include `parse_fragment_flag`** despite low pooled AUC — it carries inverted signal (controls > breaks) which a tree model can exploit.

---

## Outputs

- This memo: `docs/research/evidence/era14-phase2-feature-auc.md`
- Analysis script (one-shot, not committed): `/tmp/era14_phase2_analysis.py`
- Inputs unmodified.
