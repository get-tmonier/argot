# Era 14 — ML Stage: Status & Findings to Date

> **Status**: PAUSED, NOT CLOSED. Era 14 is open to further attempts. The current
> branch (`feat/era-14-ml-stage`) carries the leak-fixed feature extractor and
> infrastructure that any future ML attempt should build on. Production scorer
> is unchanged.

---

## TL;DR

Era 14 set out to add a 4th ML stage that catches the 5 still-missed faker-js
fixtures era 11 left behind (`error_flip_2/3`, `runtime_fetch_1/2/3` —
callees attested in their cluster, so cluster-conditional scoring contributes 0).

We built a feature extractor over the existing 3 stages, ran a 5-phase
investigation with pre-registered kill-switches, and found two distinct
classes of label leakage in the bench data when used for ML training:

1. **Routing leak** (Phases 3–3.5): catalog fixtures use synthetic
   `breaks/break_*.{ts,py}` paths that aren't in `model_a_files`, so they hit
   the cluster-assignment fallback path while controls hit the static path. A
   single binary feature (`cluster_assignment_method == "fallback_jaccard"`)
   got AUC **0.9533** for predicting `is_break`. Phase 3's apparent CV AUC
   0.9998 was 95% routing shortcut.
2. **Fixture-shape leak** (Phase 3.6b): catalog break files are single-function
   standalone files where `hunk_callees ⊆ file_callees` by construction
   (`hunk_file_callee_jaccard ≈ 1`). Real-PR controls span large multi-function
   files (`jaccard ≈ 0.05`). A single continuous feature gets AUC **0.844** for
   predicting `is_break` even after the routing fix.

We fixed the routing leak (`b74aed7`: unified Jaccard routing for ML feature
extraction; production scorer unchanged). The fixture-shape leak is documented
but not addressed — fixing it requires either redesigning the catalog format
(catalog fixtures specify a real corpus host file) or adopting sub-hunk control
sampling. Both are substantial.

After the routing fix, the conservative XGBoost model (engineered features
only, no leakage proxies) achieves Set A CV AUC **0.981**, Set B CV AUC **0.872**
— good aggregate discrimination — but **catches 0 of 5 residual faker-js
fixtures at the era-11 baseline FP rate**. The era's motivating signal is
not there in the conservative features.

---

## What's in the branch

| Item | Path | Status |
|---|---|---|
| Feature extractor (CLI) | `engine/argot/ml/cli.py`, `engine/argot/ml/features.py` | Committed `8f52eb9`, fixed `b74aed7` |
| Unified Jaccard routing for ML | `engine/argot/scoring/scorers/call_receiver.py::force_jaccard_routing` | Committed `b74aed7`, opt-in flag, production untouched |
| Subprocess-per-corpus extractor | `engine/argot/ml/cli.py::_run_all` | Committed `b74aed7`, bounds RAM |
| Phase 3 training script | `engine/argot/ml/train.py` | Committed (research-only, excluded from lint/typecheck) |
| Tests for ML features + clustering routing | `engine/argot/tests/test_ml_features.py`, `test_call_receiver_clustering.py` | Committed |
| Re-extracted feature data | `engine/.era14-features/*.jsonl` | Gitignored (1.9k rows × 6 corpora; regen via CLI) |
| Saved XGBoost models | `.era14-features/pooled_*.joblib` | Gitignored (regen via train.py) |
| Phase 2/3/3.5/3.6b memos | `docs/research/evidence/era14-phase*.md` | Committed |

---

## Phase-by-phase summary

### Phase 1 — Feature extractor (commit `8f52eb9`)

Built `argot-extract-features` CLI emitting per-hunk feature vectors as JSONL.
~20 numeric features per hunk: existing-stage outputs (`bpe_score`,
`adjusted_bpe`, `n_unattested_callees`, `n_cluster_absent_callees`,
`stage*_flagged`, `import_score`), call-receiver derivatives, hunk-vs-file
context (`hunk_file_callee_jaccard`, `hunk_callees_in_file_fraction`), hunk
shape (length, AST node-type histogram, nesting, returns/throws/awaits,
`parse_fragment_flag`), cluster routing metadata. 17 unit tests; CLI smoke test
passes.

### Phase 2 — Per-feature AUC (no model)

Pre-registered kill-switch: at least one feature must reach pooled AUC > 0.55.
**Result: PASS**. Top features by pooled AUC: `adjusted_bpe` 0.932,
`cluster_jaccard_to_centroid` 0.912, `n_unattested_callees` 0.854, `bpe_score`
0.833. 5 features cleared the stricter "AUC > 0.55 on every individual corpus"
bar.

Memo: [`era14-phase2-feature-auc.md`](era14-phase2-feature-auc.md)

### Phase 3 — Pooled XGBoost (no LOO yet)

Pre-registered gates: Set A CV AUC > 0.85, Set B CV AUC > 0.70. **Result:
PASS** at suspiciously high values. Set A AUC **0.9991**, Set B AUC **0.9998**,
residual catch **5/5**. The result was too good to be true; orchestrator
flagged for leakage investigation before proceeding.

Memo: [`era14-phase3-pooled-xgboost.md`](era14-phase3-pooled-xgboost.md)

### Phase 3.5 — Leakage probe

Tested whether single binary features could predict `is_break` (the
"is_catalog_file" hypothesis). **Result: ROUTING LEAK CONFIRMED**.
`cluster_assignment_method == "fallback_jaccard"` gives AUC **0.9533** alone.
The full-feature model added only 0.046 on top of this single-feature shortcut.

Conservative model (dropping all leakage-suspect features): Set A AUC drops
to 0.985, Set B AUC drops to 0.872, residual catch drops to 3/5 (with
`runtime_fetch_1` collapsing to rank 91/217 of controls).

Diagnosis: the bench data has 110/115 fixtures routing through fallback
Jaccard vs 0/1800 controls — perfect predictor of `is_break`.

Memo: [`era14-phase3.5-leakage-probe.md`](era14-phase3.5-leakage-probe.md)

### Phase 3.6a — Routing fix (commit `b74aed7`)

Added `force_jaccard_routing=True` opt-in flag to `CallReceiverScorer`.
ML feature extractor now uses unified Jaccard routing for every hunk
(catalog and control alike). The `cluster_assignment_method` field was
removed from the FeatureRow schema entirely. Production scorer unchanged.

Also addressed the 22 GB RAM bloat by switching `argot-extract-features --all`
to spawn a fresh subprocess per corpus (process death frees all transformer/
sklearn state). Smoke test confirmed the routing-leak categorical proxy
collapsed from AUC 1.0 to 0.51.

### Phase 3.6b — Re-analysis on freshly-extracted data

Re-ran Phase 2 + Phase 3 on fresh JSONL extracted with the unified routing.
Conservative AUCs unchanged from Phase 3.5 (Set A 0.981, Set B 0.872). But:

- Best single-feature `is_break` proxy fell from 0.953 (`fallback_jaccard`)
  to **0.886** (`n_unattested_callees`, an honest signal).
- Residual catch at FP ≤ 0.9%: **0/5** (down from Phase 3.5's 3/5).
- A new leak surfaced: `hunk_file_callee_jaccard` continuous, AUC 0.844 —
  a **fixture-shape** leak from catalog files being single-function standalone.
  Not addressed in this branch.

Memo: [`era14-phase3.6b-post-leak-fix.md`](era14-phase3.6b-post-leak-fix.md)

---

## Pre-registered gates: where we stand

| Gate | Threshold | Result | Pass |
|---|---|---|---|
| 1 (Phase 2) | Any feature AUC > 0.55 pooled | best 0.932 (or 0.886 honest) | ✓ |
| 2 (Phase 3) | Set A CV AUC > 0.85 | 0.981 | ✓ |
| 3 (Phase 3) | Set B CV AUC > 0.70 | 0.872 | ✓ |
| 4 (Phase 4) | LOO held-out AUC > existing+0.05 on ≥4 of 6 splits | NOT RUN | — |
| 5 (Phase 5) | ≥2 of 5 residual faker-js fixtures caught at faker-js FP ≤ 0.9% | **0/5** | ✗ |

Aggregate discrimination passes; the era's motivating fixture-catch test
fails. Phase 4 (LOO) was not run because Phase 5 already failed in preview
(0/5 residual catch under the conservative model is unlikely to recover with
held-out cross-validation).

---

## Why the era didn't ship (yet)

The bench data was designed for rule-based scoring where catalog fixtures
are self-contained synthetic files demonstrating one break pattern each.
The rule-based scorer doesn't notice file-shape distribution because its
features are derived per-hunk against the corpus's own statistics.

ML notices file-shape distribution because file-shape features are the
strongest discriminators in the data — and they don't transfer to real-world
hunks. The "0/5 residual catch" tells us the residual fixtures' actual
anomaly signal (their semantic break) is *not* what the model is learning;
the model is learning catalog-vs-real artifacts.

---

## What a future ML attempt would need to fix first

Before any further training:

1. **Address the fixture-shape leak.** Two options:
   - Catalog format change: each fixture specifies a real corpus host file
     (e.g. `host_file: src/modules/person/person.ts`) and the fixture's hunk
     gets injected into that host's content for scoring. Catalog fixtures
     would then have realistic file context and `hunk_file_callee_jaccard`
     would no longer be a leak proxy.
   - Sub-hunk control sampling: sample controls as small slices of real PR
     files matched in shape to catalog fixtures, so controls and fixtures
     share file-shape distribution.

2. **Verify the leak is gone with the same single-feature test**: re-train
   binary classifiers on each suspect feature and confirm none reach AUC > 0.7
   alone.

3. **Re-run the Phase 1–3.6b sequence with the fixed data**, then proceed to
   Phase 4 (LOO) only after the residual catch preview passes.

4. **Pre-register an even stricter shipping gate**: catches ≥2 of 5 residuals
   AND zero recall regression on 111 other fixtures AND zero FP-rate increase
   on any corpus — same as the original era-14 spec.

Without (1), no amount of model tuning will produce a generalizable ML stage.

---

## Honest read on what we learned

- **The 11-era statistical pipeline is robust to ML's traps**: the rule-based
  features didn't accidentally encode catalog-vs-real distinctions. ML did,
  immediately.
- **AUC 0.998 should always be treated as guilty until proven innocent.** The
  Phase-3.5 single-feature proxy test (one binary feature → AUC 0.95) is a
  cheap leak detector worth running before any pooled-CV result is trusted.
- **The bench was not built for ML.** Rule-based eval is happy with synthetic
  catalog files; ML eval needs catalog and control hunks to share the same
  data-generating process. Fixing this is a pre-requisite, not a tweak.
- **The negative results are evidence**, not waste. Era 1 plateaued at AUC 0.58
  on synthetic mutations and we never understood why structurally; era 14
  identified two specific structural causes (routing + fixture shape) that
  any future attempt has to address.

---

## Provenance

- Phase 2: [`era14-phase2-feature-auc.md`](era14-phase2-feature-auc.md)
- Phase 3: [`era14-phase3-pooled-xgboost.md`](era14-phase3-pooled-xgboost.md)
- Phase 3.5: [`era14-phase3.5-leakage-probe.md`](era14-phase3.5-leakage-probe.md)
- Phase 3.6b: [`era14-phase3.6b-post-leak-fix.md`](era14-phase3.6b-post-leak-fix.md)
- Branch: `feat/era-14-ml-stage`, commits `8f52eb9` (extractor), `b74aed7` (routing+RAM fix)
- Era-11 baseline (the production scorer this would have augmented): [`../11-cluster-conditional-attestation.md`](../11-cluster-conditional-attestation.md)

## End of Document
