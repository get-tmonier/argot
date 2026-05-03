# Era 12 — Phase 3.5: Label-leakage probe (KILL-SWITCH ON PHASE 3)

**Date**: 2026-05-03
**Branch**: `docs/era-10-root-readme` (analysis-only)
**Inputs**: `engine/.era12-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl`
**Code**: `/tmp/era12_phase35_probe.py` (one-shot, not committed)
**Artifacts**: `.era12-features/{pooled_conservative.joblib, feature_list_conservative.json}`

---

## TL;DR

| Test | Value |
|---|---:|
| **Single-feature `is_catalog_file` 5-fold CV AUC for predicting `is_break`** | **0.9533** |
| Conservative-feature Set A 5-fold CV AUC (was 0.9991) | 0.9847 |
| Conservative-feature Set B 5-fold CV AUC (was 0.9998) | 0.8717 |
| Verdict | **MOSTLY LEAK, SOME SIGNAL** |

The Phase-3 model was learning a near-perfect routing shortcut: catalog break files are routed through `fallback_jaccard` clustering; real PR controls are routed through `static_corpus`. A model that knows nothing else but "is this a fallback_jaccard row" gets AUC 0.9533 on the labels — that is the upper bound on how much of Phase 3's 0.9991 was real signal.

That said, the conservative model retains AUC 0.8717 on Set B (gate was 0.70), so there *is* some real anomaly signal beneath the leak. Era 12 should proceed to Phase 4 LOO with the conservative feature list, **and** the data pipeline must be redesigned so catalog fixtures share the same routing path as controls before the production scorer is built on top.

---

## The leakage mechanism — measured

The hypothesis was that catalog fixtures (under `benchmarks/catalogs/<corpus>/breaks/break_*.{ts,py}`) get a different cluster-assignment path than real PR hunks. The data confirms this *almost perfectly*:

```
                          is_break=True    is_break=False
fallback_jaccard               110               90
static_corpus                    0             1552
none                             5              158
```

- **110 / 115 (96%) of breaks** route through `fallback_jaccard`.
- **0 / 1800 controls** route through `static_corpus` AND are also breaks.
- The remaining 5 breaks (`ink_class_components_{1,2,3,4}` + `ink_lifecycle_2`) get `"none"` (parse failure) — still distinct from the dominant control routing.
- The 90 catalog-routed *non-breaks* are all from the `faker` corpus's i18n provider files (e.g. `faker/providers/automotive/zh_CN/__init__.py`) — these are real PR controls that happen to fall back to jaccard clustering because they're tiny stub files. That's the only thing keeping the single-feature AUC at 0.953 instead of 1.000.

In Set B specifically the leak is even tighter: **12/12** breaks are `fallback_jaccard`, **17/1441** controls are. The Phase-3 Set B AUC of 0.9998 is almost entirely explained by this one categorical.

---

## Task 1 — Per-feature AUC for `is_catalog_file`

Sorted by `is_catalog_file` AUC desc. AUC reported as `max(auc, 1-auc)` so direction doesn't matter for the leakage diagnosis.

| Feature | AUC is_catalog_file | AUC is_break | Suspicion |
|---|---:|---:|---|
| adjusted_bpe | 0.9233 | 0.9322 | STRONG LEAK |
| cluster_jaccard_to_centroid | 0.9059 | 0.9125 | STRONG LEAK |
| hunk_file_callee_jaccard | 0.8803 | 0.8859 | PARTIAL LEAK |
| n_cluster_absent_callees | 0.8151 | 0.7760 | PARTIAL LEAK |
| stage2_flagged | 0.7952 | 0.7655 | PARTIAL LEAK |
| hunk_callee_bag_size | 0.7501 | 0.7882 | PARTIAL LEAK |
| n_distinct_callees | 0.7501 | 0.7882 | PARTIAL LEAK |
| hunk_callees_in_file_fraction | 0.7490 | 0.7546 | PARTIAL LEAK |
| n_unattested_callees | 0.7477 | 0.8539 | PARTIAL LEAK |
| ast_argument_list | 0.7461 | 0.6651 | PARTIAL LEAK |
| ast_ERROR | 0.7436 | 0.7644 | PARTIAL LEAK |
| ast_call | 0.7402 | 0.6711 | PARTIAL LEAK |
| n_returns | 0.7389 | 0.7043 | PARTIAL LEAK |
| ast_attribute | 0.7341 | 0.6390 | PARTIAL LEAK |
| bpe_score | 0.7309 | 0.8328 | PARTIAL LEAK |
| ast_string_content | 0.7273 | 0.6403 | PARTIAL LEAK |
| ast_string_end | 0.7236 | 0.6351 | PARTIAL LEAK |
| ast_string_start | 0.7110 | 0.6189 | PARTIAL LEAK |
| ast_type | 0.7095 | 0.6735 | PARTIAL LEAK |
| max_nesting_depth | 0.7089 | 0.7495 | PARTIAL LEAK |
| file_callee_bag_size | 0.6896 | 0.6626 | OK |
| n_total_ast_nodes | 0.6810 | 0.6551 | OK |
| parse_fragment_flag | 0.6731 | 0.6844 | OK |
| n_attested_root_only | 0.6640 | 0.7069 | OK |
| ast_string | 0.6608 | 0.6351 | OK |
| ast_expression_statement | 0.6589 | 0.6368 | OK |
| ast_assignment | 0.6578 | 0.5913 | OK |
| ast_identifier | 0.6503 | 0.6222 | OK |
| hunk_length_chars | 0.6404 | 0.5954 | OK |
| cluster_id | 0.6305 | 0.6398 | OK |
| hunk_length_lines | 0.6243 | 0.5684 | OK |
| ast_string_fragment | 0.5895 | 0.5189 | OK |
| n_distinct_identifiers | 0.5829 | 0.6168 | OK |
| ast_pair | 0.5708 | 0.6323 | OK |
| ast_property_identifier | 0.5673 | 0.5416 | OK |
| n_throws | 0.5557 | 0.5941 | OK |
| ast_type_identifier | 0.5478 | 0.5106 | OK |
| import_score | 0.5475 | 0.5826 | OK |
| stage1_flagged | 0.5475 | 0.5826 | OK |
| ast_member_expression | 0.5368 | 0.5597 | OK |
| ast_type_annotation | 0.5316 | 0.5224 | OK |
| ast_arguments | 0.5193 | 0.5712 | OK |
| ast_object | 0.5081 | 0.5392 | OK |
| n_awaits | 0.5043 | 0.5204 | OK |

Notes:
- 20 features score >0.70 on `is_catalog_file`. Their AUC for `is_break` is closely tracked by their AUC for `is_catalog_file` — i.e. they are mostly proxies for the routing distinction.
- The two top Phase-3 features in the original memo (`adjusted_bpe`, `cluster_jaccard_to_centroid`) are STRONG LEAK by this measure.
- `ast_string_fragment`, the surprise #1 splitter for Set B in Phase 3, scores **AUC 0.5895** for `is_catalog_file`. It's *not* a catalog routing proxy, which is consistent with the Phase-3 hypothesis that XGBoost found a real TS-template-string interaction. But the catalog routing dominates the loss surface, so this finding may not survive once routing is removed (see Task 2).

---

## Task 2 — Conservative re-train

**Conservative feature set construction.** Per task spec, KEEP stage outputs + simple hunk-shape features; DROP cluster routing, AST families (other than `n_total_ast_nodes` if it survives), file-shape, and file-callee-fraction features. Then additionally drop any feature with `is_catalog_file` AUC > 0.85.

Pre-filter spec (13): `bpe_score, adjusted_bpe, n_unattested_callees, n_cluster_absent_callees, stage2_flagged, import_score, cluster_id, n_distinct_callees, max_nesting_depth, n_returns, n_throws, n_awaits, parse_fragment_flag`

Post-filter (12, dropped `adjusted_bpe` for AUC 0.92 leakage):

```
bpe_score, n_unattested_callees, n_cluster_absent_callees, stage2_flagged,
import_score, cluster_id, n_distinct_callees, max_nesting_depth,
n_returns, n_throws, n_awaits, parse_fragment_flag
```

`n_total_ast_nodes` — also dropped per spec (catalog file size leak), even though its `is_catalog_file` AUC is 0.68.

**Results (XGBoost with pre-registered hyperparameters, 5-fold stratified CV):**

| Set | Total | Breaks | Phase-3 AUC | Conservative AUC | Δ |
|-----|------:|-------:|------------:|-----------------:|---:|
| A | 1,915 | 115 | 0.9991 | **0.9847 ± 0.0106** | −0.014 |
| B | 1,453 | 12  | 0.9998 | **0.8717 ± 0.0971** | −0.128 |

Per-fold:
- Set A: 0.9756, 0.9919, 0.9957, 0.9686, 0.9919
- Set B: 0.9387, 0.9572, 0.8028, 0.7135, 0.9462 — note the high variance (one fold at 0.71)

**Top-5 features by gain (Set A — conservative):**

| Rank | Feature | Gain |
|---:|---|---:|
| 1 | n_unattested_callees | 10.94 |
| 2 | bpe_score | 6.66 |
| 3 | n_cluster_absent_callees | 4.19 |
| 4 | parse_fragment_flag | 3.24 |
| 5 | max_nesting_depth | 3.18 |

**Top-5 features by gain (Set B — conservative):**

| Rank | Feature | Gain |
|---:|---|---:|
| 1 | n_throws | 2.52 |
| 2 | cluster_id | 2.00 |
| 3 | max_nesting_depth | 1.73 |
| 4 | n_returns | 1.13 |
| 5 | parse_fragment_flag | 1.07 |

The Set A top-5 reads like a sane anomaly-detection feature list (BPE surprise + cluster-absent callees + nesting). The Set B top-5 is more brittle (n_throws/n_returns/cluster_id at low gain values, indicating the model is grasping at small distributional differences with only 12 positives) — this matches the fold variance and the residual-fixture collapse in Task 4.

---

## Task 3 — Verdict: **MOSTLY LEAK, SOME SIGNAL**

Per pre-registered interpretation:

| Bucket | Set A | Set B | Observed |
|---|---|---|---|
| GENUINE SIGNAL | > 0.85 | > 0.70 | A=0.985 ✓, B=0.872 ✓ |
| MOSTLY LEAK, SOME SIGNAL | 0.65-0.85 | 0.55-0.70 | — |
| MOSTLY LEAK | ≈0.55 | ≈0.55 | — |

By the strict letter of the verdict bands, conservative AUCs **clear the GENUINE SIGNAL bar**. But a single binary feature (`is_catalog_file`) reaches AUC 0.9533 on the same labels — i.e. *most* of the discriminative power in the entire featurization is the routing distinction, not the anomaly signal. The conservative model's residual-fixture collapse (Task 4) reinforces this: when we strip the leak proxies, the model ranks the residuals as ordinary controls.

I'm calling **MOSTLY LEAK, SOME SIGNAL** rather than GENUINE SIGNAL because:
1. The single-feature smoke test says the catalog routing alone is more discriminative than every individual conservative feature.
2. Set B fold variance jumped from σ=0.0005 to σ=0.0971 — the conservative model is unstable on the regime that actually matters.
3. The 5/5 residual catch claim from Phase 3 evaporates (see Task 4) — only 1 of 5 still scores above all controls.

Era 12 should proceed to Phase 4 (LOO) with the conservative feature list as the primary candidate, but **the data pipeline needs a fix**. Either reroute catalog fixtures through `static_corpus` clustering, or move controls into the same `fallback_jaccard` path, before any model is taken seriously.

---

## Task 4 — Residual fixture re-prediction (conservative Set B model, OOF)

| Fixture | Phase 3 prob | Conservative prob | Phase 3 rank | Conservative rank | controls above conservative p |
|---|---:|---:|---:|---:|---:|
| faker_js_error_flip_2 | 0.9298 | 0.3990 | 5 | 1 | 0/217 |
| faker_js_runtime_fetch_2 | 0.9400 | 0.1913 | 2 | 2 | 0/217 |
| faker_js_error_flip_3 | 0.9322 | 0.1205 | 3 | 3 | 0/217 |
| faker_js_runtime_fetch_3 | 0.9314 | 0.0263 | 4 | 4 | 12/217 |
| faker_js_runtime_fetch_1 | 0.9626 | 0.0013 | 1 | 5 | 90/217 |

**Catch rate at the same Phase-3 threshold (faker-js FP ≤ 0.9%): 3/5** (top three sit above all 217 faker-js controls). The fourth has 12 controls above it (5.5% FP — well above the operating point). The fifth (`runtime_fetch_1`) is now ranked at the 41st-percentile of faker-js controls — the model believes it's barely distinguishable from a random control hunk.

The 5/5 catch from Phase 3 was a leak artifact. The conservative model recovers 3/5 with confidence but loses the other two completely.

---

## Task 5 — Path-pattern smoke test (the decisive proof)

Trained an XGBoost classifier on a **single binary feature** = `cluster_assignment_method == "fallback_jaccard"`. Same hyperparameters, same 5-fold stratified CV, predicting `is_break`.

```
5-fold CV AUC: mean=0.9533  std=0.0135
folds: 0.9764, 0.9477, 0.9574, 0.9491, 0.9357
```

Contingency:

|                       | is_break=1 | is_break=0 |
|-----------------------|-----------:|-----------:|
| `fallback_jaccard`=1  |        110 |         90 |
| `fallback_jaccard`=0  |          5 |       1710 |

This single bit explains AUC 0.9533. The full 25-feature Phase-3 model got 0.9991. **Marginal gain from 24 additional features over the routing bit: 0.046 AUC.** The interpretation is unambiguous: most of Phase-3's learning was a trivial routing shortcut.

The 5 `is_break=1` rows on `fallback_jaccard=0` are the ink class-components fixtures that fail TSX parsing. The 90 `is_break=0` rows on `fallback_jaccard=1` are short faker i18n provider files (e.g. `faker/providers/date_time/ta_IN/__init__.py`) — the only thing keeping this from being a perfect 1.0 AUC.

---

## What needs to change

1. **Unify the routing path before any future training.** Catalog fixtures and real-PR controls must hit `static_corpus` clustering in the same way, OR the production pipeline must accept that "catalog file" is a feature it relies on (which is unacceptable — it doesn't generalise to live PRs).
2. **Re-extract features after the routing fix** before re-running Phase 3 / Phase 4. The current `engine/.era12-features/*.jsonl` cannot be salvaged for honest training.
3. **Phase 4 (LOO) should still run on the conservative feature list** as a probe — if conservative LOO holds together (Set A LOO AUC > 0.7 on every held-out corpus), that's evidence of real residual signal independent of routing. Treat any LOO Set B numbers with the same skepticism this memo applies to Phase 3 Set B.
4. **Don't ship `adjusted_bpe`-driven scoring as evidence of "ML works".** The Phase-3 #1 feature was a leakage proxy with `is_catalog_file` AUC 0.92. Its discriminative power on real labels is mostly inherited from that routing shortcut.

---

## Outputs

- This memo: `docs/research/evidence/era12-phase3.5-leakage-probe.md`
- Conservative model: `.era12-features/pooled_conservative.joblib` (contains `model_a`, `model_b`, `feature_names`, CV AUC dicts)
- Conservative feature manifest: `.era12-features/feature_list_conservative.json`
- Analysis script: `/tmp/era12_phase35_probe.py` (one-shot, not committed)
- Raw run log: `/tmp/era12_phase35_run.log`
- Persisted results JSON: `/tmp/era12_phase35_results.json`

## What I couldn't analyse

- **No "honest" Phase 4 LOO run.** Spec restricted me to the conservative model + the single-feature smoke test (Tasks 2 + 5 only), not LOO. Phase 4 with the conservative feature list is the appropriate next step — recommended but not executed here.
- **No re-extraction of features under unified routing.** That requires touching the feature extractor, which the spec prohibits. The MOSTLY LEAK SOME SIGNAL verdict assumes the routing fix is a future task, not done in this probe.
