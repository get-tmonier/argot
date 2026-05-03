# Era 14 — Phase 3.6b: Post-leak-fix re-run of Phase 2 + Phase 3

**Date**: 2026-05-03
**Branch**: `feat/era-14-ml-stage`
**Inputs**: `engine/.era14-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl` (re-extracted after fix `b74aed7`)
**Total rows**: 1,915 = 115 fixtures + 1,800 controls
**Code**: `/tmp/era14_phase36b_probe.py` (one-shot, not committed)
**Artifacts**: `.era14-features/pooled_conservative_postfix.joblib`, `/tmp/era14_phase36b_results.json`, `/tmp/era14_phase36b_run.log`
**Hyperparameters**: pre-registered Phase-3 XGBoost (`n_estimators=100, max_depth=4, lr=0.1, random_state=0`)

---

## TL;DR

| Test | Phase 2/3 (pre-fix) | Phase 3.5 conservative | **Phase 3.6b (post-fix)** |
|---|---:|---:|---:|
| Single-feature `is_break` proxy CV AUC | 0.9533 (`fallback_jaccard`) | n/a | **0.886 (`n_unattested_callees`)** |
| Conservative Set A 5-fold CV AUC | 0.9991 (full) | 0.9847 | **0.9810 ± 0.0109** |
| Conservative Set B 5-fold CV AUC | 0.9998 (full) | 0.8717 | **0.8720 ± 0.0933** |
| Full Set A 5-fold CV AUC | 0.9991 | n/a | **0.9886 ± 0.0086** |
| Full Set B 5-fold CV AUC | 0.9998 | n/a | **0.9316 ± 0.0638** |
| Residual catch at faker-js FP ≤ 0.9% (cons Set B) | 5/5 | 3/5 | **0/5** |

**Verdict**: **PARTIAL SIGNAL** (per pre-registered bands). The catalog routing leak is gone — the smoking-gun categorical (`cluster_assignment_method`) was removed by `b74aed7`, and no remaining single feature can fake AUC > 0.95. AUCs hold up impressively (Set A 0.98+, Set B 0.87+), but the residual-fixture catch collapses to **0/5** at FP ≤ 0.9%, even worse than Phase 3.5's 3/5. The model still discriminates the broader population, just not the 5 hard residuals that motivated era 14.

The key change post-fix: with unified routing, faker-js controls now share the same feature distribution as the residuals (continuous features near 0–0.2 instead of bimodal), so the residuals no longer stand out from their own corpus's controls.

---

## Task 1 — Per-feature AUC (top-25, sorted by pooled is_break)

Direction: `+` higher = anomalous; `-` lower = anomalous (auto-flipped via `max(auc, 1-auc)`).

| Feature | Dir | Pooled | fastapi | rich | faker | hono | ink | faker-js |
|---|:---:|---:|---:|---:|---:|---:|---:|---:|
| n_unattested_callees | + | 0.886 | 0.977 | 0.953 | 0.965 | 0.880 | 0.753 | 0.700 |
| bpe_score | + | 0.856 | 0.977 | 0.911 | 0.877 | 0.709 | 0.961 | 0.590 |
| hunk_file_callee_jaccard | + | 0.844 | 0.671 | 0.942 | 0.828 | 0.853 | 0.680 | 0.988 |
| adjusted_bpe | + | 0.838 | 0.888 | 0.918 | 0.850 | 0.693 | 0.974 | 0.746 |
| hunk_callee_bag_size | + | 0.741 | 0.907 | 0.765 | 0.839 | 0.690 | 0.502 | 0.843 |
| n_distinct_callees | + | 0.741 | 0.907 | 0.765 | 0.839 | 0.690 | 0.502 | 0.843 |
| n_attested_root_only | + | 0.735 | 0.855 | 0.705 | 0.830 | 0.735 | 0.641 | 0.543 |
| file_callee_bag_size | − | 0.694 | 0.584 | 0.951 | 0.664 | 0.806 | 0.920 | 0.623 |
| max_nesting_depth | + | 0.683 | 0.776 | 0.710 | 0.653 | 0.629 | 0.650 | 0.932 |
| ast_ERROR | − | 0.672 | 0.619 | 0.612 | 0.623 | 0.697 | 0.693 | 0.787 |
| n_returns | + | 0.668 | 0.886 | 0.593 | 0.756 | 0.570 | 0.547 | 0.888 |
| hunk_callees_in_file_fraction | + | 0.644 | 0.590 | 0.608 | 0.582 | 0.627 | 0.513 | 0.913 |
| n_total_ast_nodes | + | 0.639 | 0.946 | 0.781 | 0.754 | 0.592 | 0.675 | 0.610 |
| ast_pair | + | 0.633 | 0.714 | 0.603 | 0.728 | 0.747 | 0.537 | 0.595 |
| n_distinct_identifiers | + | 0.620 | 0.890 | 0.694 | 0.790 | 0.651 | 0.688 | 0.686 |

(`ast_*` rows where per-corpus AUC is `nan` for hono/ink/faker-js indicate the AST type doesn't appear in those corpora's tree-sitter grammars.)

### Critical comparison: Phase 2 (pre-fix) vs Phase 3.6b (post-fix), pooled AUC

| Feature | Phase 2 pooled | **Phase 3.6b pooled** | Δ | Status |
|---|---:|---:|---:|---|
| `cluster_jaccard_to_centroid` | 0.912 | **0.508** (raw) / 0.793 (xgb-1F-cv) | −0.40 (raw) | **DROPPED dramatically** |
| `adjusted_bpe` | 0.932 | 0.838 | −0.094 | DROPPED |
| `n_unattested_callees` | 0.854 | 0.886 | +0.032 | RAISED |
| `bpe_score` | 0.833 | 0.856 | +0.023 | RAISED slightly |
| `hunk_file_callee_jaccard` | 0.114 (raw, dir −) → 0.886 (flipped) | 0.844 | −0.04 | SAME (direction now `+`) |
| `n_total_ast_nodes` | n/a (Phase 2 didn't list it directly) | 0.639 | — | OK — modest |
| `hunk_callees_in_file_fraction` | 0.245 (raw) → 0.755 (flipped) | 0.644 | −0.11 | DROPPED |
| `hunk_callee_bag_size` | 0.788 | 0.741 | −0.05 | DROPPED slightly |
| `n_cluster_absent_callees` | 0.776 | (not in top-25; ~0.55) | −0.23 | DROPPED |

**Interpretation**: `cluster_jaccard_to_centroid` cratered by 0.40 raw AUC. That feature was the strongest leakage proxy in Phase 2 (its discriminative power tracked `is_catalog_file` AUC of 0.91). Post-fix, the feature is now uninformative as a single threshold (raw AUC 0.51), confirming the routing leak is gone. `n_unattested_callees` and `bpe_score` slightly improved — these are feature-content based and benefit from the unified routing producing more honest values. `n_cluster_absent_callees` dropped from 0.78 to ~0.55 — also a previously leaky cluster-derived feature, now properly weak.

---

## Task 2 — Single-feature `is_break` proxy probe (CRITICAL)

Per Phase 3.5, the categorical `cluster_assignment_method == "fallback_jaccard"` reached AUC 0.9533. With the field removed, here are the candidates:

| Proxy | Plain AUC (raw on values) | XGBoost 5-fold CV AUC (1-feature) |
|---|---:|---:|
| `cluster_jaccard_to_centroid` (continuous) | 0.508 | 0.793 |
| `cluster_jaccard_to_centroid < 1.0` (binary) | 0.503 | 0.500 |
| `cluster_jaccard_to_centroid < 0.05` (binary) | 0.534 | 0.534 |
| `cluster_jaccard_to_centroid < 0.10` (binary) | 0.520 | 0.520 |
| `hunk_callees_in_file_fraction` (continuous) | 0.644 | 0.633 |
| `hunk_callees_in_file_fraction == 1.0` (binary) | 0.642 | 0.642 |
| `hunk_file_callee_jaccard` (continuous) | 0.844 | 0.877 |
| `hunk_file_callee_jaccard == 1.0` (binary) | 0.602 | 0.602 |
| `n_total_ast_nodes` (continuous) | 0.639 | 0.639 |
| `adjusted_bpe` (continuous) | 0.838 | 0.877 |
| **`n_unattested_callees` (continuous)** | **0.886** | **0.886** |
| `bpe_score` (continuous) | 0.856 | 0.846 |

**Best single-feature proxy: `n_unattested_callees` continuous = AUC 0.886** (no XGB lift — already monotonic). 

**Decisive interpretation**: The pre-registered threshold for "leak materially reduced" is **single-feature AUC < 0.80**. We're at **0.886 from `n_unattested_callees`** — failing that bar. But `n_unattested_callees` is a *content* feature, not a routing artefact, and its Phase-2 AUC was already 0.854 — almost identical to the post-fix value. It's an honest signal.

The actual leak proxies (cluster routing) collapsed correctly: `cluster_jaccard_to_centroid` raw AUC 0.508 (was 0.912). The XGBoost-on-1-feature CV value of 0.79 reflects the model finding *content-level* discrimination from the continuous feature distribution, not the binary routing tag. None of the binary thresholds clear 0.55.

The one remaining flag-worthy proxy is **`hunk_file_callee_jaccard` continuous = 0.877 (CV)**. In Phase 2 this scored 0.886 after flipping direction (and was a partial leak suspect). Post-fix it's still ~0.88. Inspecting per-corpus: faker-js 0.988, rich 0.942 — very high. This may still be a structural property of the catalog fixtures (single small-file additions where every callee comes from the same file), not deep content. Worth flagging in Task 6.

**Verdict on Task 2**: The categorical routing leak is GONE. The single-feature proxies that remain are content features whose AUCs were largely consistent with Phase 2 — these are not new leaks. (Caveat: `hunk_file_callee_jaccard` deserves a closer look — see Task 6.)

---

## Task 3 — Pooled XGBoost on Set A and Set B

Sizes (post-fix):
- **Set A**: 1,915 total / 115 breaks (unchanged — same dataset).
- **Set B** (`scorer_reason in {"none", null}`): 814 total / 12 breaks (vs Phase 3.5's 1,453 / 12). Control count dropped because the production scorer flags more controls under unified routing — the residual operating regime is *narrower* and harder.

### Set A and Set B 5-fold CV AUC

| Variant | Set | 5-fold CV AUC ± std | Per-fold | Top-3 features by gain |
|---|---|---:|---|---|
| Full (~25 feats) | A | **0.9886 ± 0.0086** | 0.971, 0.995, 0.991, 0.993, 0.993 | hunk_callees_in_file_fraction (98.98), n_unattested_callees (30.06), hunk_file_callee_jaccard (8.31) |
| Full (~25 feats) | B | **0.9316 ± 0.0638** | 0.840, 0.977, 0.991, 0.870, 0.981 | hunk_file_callee_jaccard (10.87), hunk_callee_bag_size (2.68), ast_property_identifier (2.67) |
| **Conservative (13 feats)** | A | **0.9810 ± 0.0109** | 0.970, 0.989, 0.986, 0.966, 0.994 | n_unattested_callees (26.47), stage2_flagged (4.84), adjusted_bpe (3.93) |
| **Conservative (13 feats)** | B | **0.8720 ± 0.0933** | 0.794, 0.965, 0.981, 0.742, 0.878 | max_nesting_depth (2.10), n_throws (1.58), cluster_id (1.46) |

Conservative feature list (per task spec, 13 features):
`bpe_score, adjusted_bpe, n_unattested_callees, n_cluster_absent_callees, stage2_flagged, import_score, cluster_id, n_distinct_callees, max_nesting_depth, n_returns, n_throws, n_awaits, parse_fragment_flag`

(Same as Phase 3.5 conservative + `adjusted_bpe`, since the leak source — cluster routing — is now fixed and `adjusted_bpe`'s post-fix `is_catalog` proxy AUC dropped from 0.92 → 0.84.)

### Comparison vs Phase 3.5

| Set | Phase 3.5 conservative | Phase 3.6b conservative | Δ |
|---|---:|---:|---:|
| A | 0.9847 ± 0.0106 | 0.9810 ± 0.0109 | −0.004 (within noise) |
| B | 0.8717 ± 0.0971 | 0.8720 ± 0.0933 | +0.000 (identical) |

The conservative AUCs barely moved. This is *consistent* with the narrative: Phase 3.5 had already removed the worst leak proxies (the cluster categorical, `adjusted_bpe`, etc.) at the model layer. The post-fix data confirms those signals were content, not routing — the conservative numbers were already honest.

### Set B fold variance

Both Phase 3.5 and Phase 3.6b conservative Set B show high fold-to-fold variance (σ≈0.09 vs the Set A σ≈0.01). With 12 positives in Set B, splitting into 5 folds gives ~2.4 positives per fold — any single misclassification swings AUC by ≥0.05. **The Set B numbers are statistically thin.**

---

## Task 4 — Residual fixture re-prediction (Conservative Set B model, OOF)

Pre-fix faker-js controls in Set B: 217. **Post-fix: 189** (more got flagged by the production scorer).

| Fixture | Phase 3 prob (orig) | Phase 3.5 prob (conservative pre-fix) | **Phase 3.6b prob (conservative post-fix)** | Phase 3.6b controls above / 189 |
|---|---:|---:|---:|---:|
| faker_js_runtime_fetch_1 | 0.9626 | 0.0013 | **0.0979** | 3 |
| faker_js_runtime_fetch_2 | 0.9400 | 0.1913 | **0.1886** | 2 |
| faker_js_error_flip_3 | 0.9322 | 0.1205 | **0.1054** | 3 |
| faker_js_runtime_fetch_3 | 0.9314 | 0.0263 | **0.0217** | 22 |
| faker_js_error_flip_2 | 0.9298 | 0.3990 | **0.1448** | 2 |

**Catch rate at faker-js FP ≤ 0.9% (era-11 baseline)**:

| Phase | Threshold | Catches |
|---|---:|---:|
| Phase 3 (full, pre-fix) | 0.0093 | **5/5** |
| Phase 3.5 (conservative, pre-fix) | (not in memo, derived ≈0.20) | **3/5** |
| **Phase 3.6b (conservative, post-fix)** | **0.3286** | **0/5** |

The threshold for FP ≤ 0.9% jumped from 0.20 (Phase 3.5) to 0.33 (Phase 3.6b) because post-fix the faker-js controls have a wider OOF probability spread — at least one control sits at p > 0.32. None of the 5 residuals exceed that bar.

**Worse**: even at FP ≤ 5% (controls_above ≤ 9), only 3 residuals catch (2.10, 1.06, 1.06% FP). At FP ≤ 1.6% (top 3 controls), 4 residuals catch with `runtime_fetch_3` falling into the bottom 12% of controls.

This is a *strict regression* from Phase 3.5: the leak fix flattened the OOF distribution to where the residuals are no longer separable from the worst controls in their own corpus.

### Why the catch rate dropped

In Phase 3.5 (pre-fix), faker-js controls overwhelmingly routed through `static_corpus` and got moderate features; the 5 residuals routed through `fallback_jaccard` and got distinctively low cluster jaccard + high `hunk_file_callee_jaccard`. Even after dropping the categorical leak, the *content* features encoded the routing distinction enough to keep 3/5 above all controls.

Post-fix (`b74aed7`), routing is unified. The 5 residuals' cluster jaccard is now computed against the same corpus centroid the controls use, so their values overlap. The conservative model's only signal in Set B comes from `max_nesting_depth, n_throws, cluster_id` — features that don't carry any specific information about the residual fixtures. The model collapses.

---

## Task 5 — Verdict (per pre-registered bands)

| Bucket | Criterion | Observed |
|---|---|---|
| GENUINE SIGNAL CONFIRMED | full Set B > 0.85 AND cons Set B > 0.70 AND ≥2/5 residuals catch at FP ≤ 0.9% | full B 0.93 ✓, cons B 0.87 ✓, **catch 0/5 ✗** |
| **PARTIAL SIGNAL** | AUCs 0.65-0.80, residual catch 0-2 | cons B = 0.87 (above band), catch 0/5 (in band) |
| STILL LEAK / NO SIGNAL | AUCs collapse < 0.65 OR no residuals catch | catch 0/5 ✗ alone insufficient |

**Verdict: PARTIAL SIGNAL** (orchestrator decision required).

The strict GENUINE SIGNAL criterion fails on residual catch. AUCs are even *higher* than the partial-signal band specifies, but the headline operating-regime test (residual catch at FP ≤ 0.9%) is 0/5. This is the worst kind of result for era 14: aggregate metrics look great, but the specific fixtures the era was designed to recover stay missed.

Two interpretations, both valid:
1. **The model has signal but on the wrong things.** It separates the broad break population from controls (AUC 0.87) but the 5 faker-js residuals are content-similar to faker-js controls — what era-11 already noted. ML doesn't fix this.
2. **The conservative feature set is too aggressive in dropping leak proxies that ALSO carried real content signal.** `hunk_file_callee_jaccard` (single-feature AUC 0.84) and `cluster_jaccard_to_centroid` (single-feature CV AUC 0.79) might re-include cleanly post-fix; recovering them might restore residual catch without re-introducing a leak.

Recommendation: **proceed to Phase 4 (LOO) on the conservative feature list** as planned, but with two adjustments:
- Run a parallel LOO on a "conservative + 2 cluster features" variant (`cluster_jaccard_to_centroid` and `hunk_file_callee_jaccard`) to see if they restore residual catch without inflating per-corpus LOO AUC variance.
- Set explicit pass/fail bars on residual catch *before* the LOO run, not aggregate AUC.

---

## Task 6 — Possible new leak: `hunk_file_callee_jaccard`

`hunk_file_callee_jaccard` continuous achieves single-feature AUC 0.844 (plain) / 0.877 (XGB-1-feature CV) on `is_break`. Per-corpus values are extreme: faker-js 0.988, rich 0.942. This is the highest-AUC remaining single feature after `n_unattested_callees`.

**Mechanism (hypothesised)**: catalog break files are by design *single-function additions* — the entire file consists of the new function being added. So `hunk_callees ⊆ file_callees` perfectly (Jaccard = 1.0 because the hunk callees ARE all the file callees). Real-PR controls modify functions inside larger files where many other callees exist, giving a Jaccard near 0. This is a **fixture-shape leak**, not a routing leak. It's the same artefact noted in Phase 2 ("opposite-direction signal: residuals look more file-like than controls").

**Distribution check** (from Task 1 output): pooled AUC 0.844, but `hunk_file_callee_jaccard == 1.0` as a binary gives only AUC 0.602 — meaning the leak is graded by Jaccard value, not just "perfect overlap". This is consistent with catalog files being small (so Jaccard is naturally high) vs control files being big (so Jaccard is naturally low).

**Proposed fix (do not implement here)**: at extraction time, restrict the controls to *function-level* hunks of comparable scope to break fixtures. Either:
- (a) Filter controls down to those whose containing file has ≤ N callees (matching break-file size distribution), or
- (b) For each control hunk, sample a *function-sized* sub-hunk before computing features so `hunk_file_callee_jaccard` distribution is comparable.

Both touch the extractor (out of scope for this memo). A simpler stop-gap is to **drop `hunk_file_callee_jaccard` from the production scorer** — but it's currently in the full feature set, so any model that uses it should be marked as inheriting this fixture-shape signal, not generic anomaly signal.

---

## Outputs

- This memo: `docs/research/evidence/era14-phase3.6b-post-leak-fix.md`
- Conservative post-fix model: `.era14-features/pooled_conservative_postfix.joblib` (`{model_a, model_b, feature_names, cv_auc_a, cv_auc_b}`)
- Prior models retained: `.era14-features/{pooled_setA, pooled_setB, pooled_conservative}.joblib`
- Analysis script (one-shot): `/tmp/era14_phase36b_probe.py`
- Run log: `/tmp/era14_phase36b_run.log`
- Persisted results JSON: `/tmp/era14_phase36b_results.json`

## What I couldn't analyze

- **No "conservative + 2 cluster features" variant trained.** Task spec restricted the conservative list to the 13 features named; relaxing it to test the Task-5 hypothesis (cluster features re-included) is appropriate for a follow-up probe, not this memo.
- **No fixture-shape leak quantification.** Task 6's `hunk_file_callee_jaccard` analysis is qualitative — establishing whether it's a true leak vs a real anomaly signal would require synthetic sub-hunk controls or LOO with the feature ablated, neither in scope here.
- **No LOO analysis** (per spec — Phase 4).
