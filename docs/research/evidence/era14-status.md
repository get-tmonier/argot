# Era 14 — ML Stage: Status & Findings to Date

> **Status**: CLOSED NEGATIVE — six major ML approaches tested across
> the embedding-anomaly axis, including the principled clean ablation
> (Phase 7.1) that isolates PCA-whitening + per-cluster anchoring from
> Phase 7's broken per-cluster Σ. Phase 7.1 carries 0/5 residuals at a
> clean LOO sanity check (max ratio 1.56× ≈ (k/(k-1))² for k=5, the
> expected small-sample mean shift). Combined with Phase 7's LOO
> finding (per-cluster Σ on k<d controls is rank-deficiency artifact),
> these two phases close the embedding-anomaly axis honestly: the
> residuals are typical-looking under any covariance-respecting
> metric, and Phase 6.4's cosine-axis 1/5 catch (`runtime_fetch_2`)
> remains the ceiling. The current branch (`feat/era-14-ml-stage`)
> carries the leak-fixed feature extractor, host-file injection for
> all 115 fixtures, streaming RAM-bounded extraction, UnixCoder
> embedding support, cache-first HF loading, and saved models —
> substantial infrastructure for any future attempt. Production
> scorer is unchanged.
>
> Best honest result: **Phase 6.4 caught 1/5 residuals**
> (`runtime_fetch_2`) at faker-js FP ≤ 0.9% via unsupervised
> cluster-centroid cosine distance. SHIP gate (≥2/5) not met by any
> phase under proper LOO + FP budget. Era 14 closes on Phase 6.4's
> PARTIAL after the principled refinement (Phase 7.1) confirmed the
> ceiling is not a tuning artifact.

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
| Feature extractor (CLI) | `engine/argot/ml/cli.py`, `engine/argot/ml/features.py` | Committed `8f52eb9`, fixed `b74aed7`, hosts `2cb3e27` |
| Unified Jaccard routing for ML | `engine/argot/scoring/scorers/call_receiver.py::force_jaccard_routing` | Committed `b74aed7`, opt-in flag, production untouched |
| Subprocess-per-corpus extractor | `engine/argot/ml/cli.py::_run_all` | Committed `b74aed7`, bounds RAM |
| Streaming control sampling | `engine/argot/ml/cli.py::stream_sample_controls` | Committed `2c8dcc4`, min-heap top-N + reservoir, RAM bounded |
| Host-file injection (Fix A) | `benchmarks/src/argot_bench/fixtures.py`, `engine/argot/ml/features.py::synthesize_hunk_in_host` | Committed `2cb3e27`, all 115 fixtures have hosts |
| Phase 3 training script | `engine/argot/ml/train.py` | Committed (research-only, excluded from lint/typecheck) |
| Tests for ML features + clustering routing | `engine/argot/tests/test_ml_features.py`, `test_call_receiver_clustering.py` | Committed (221 tests pass) |
| Re-extracted feature data (Fix-A) | `engine/.era14-features/*.jsonl` | Gitignored (1.9k rows × 6 corpora; regen via CLI) |
| Saved XGBoost models | `.era14-features/pooled_*.joblib`, `loo_models_fixA/*.joblib` | Gitignored (regen via train.py) |
| All evidence memos | `docs/research/evidence/era14-*.md` | Committed |

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

### Phase 4 — Fix A pilot (faker-js only)

After 115 fixtures got `host_file` + `host_inject_at_line` fields and the
extractor was modified to splice catalog content into the host file before
scoring, the faker-js pilot ran. Results:

- `hunk_file_callee_jaccard` AUC dropped from 0.988 → 0.908 — fixture-shape
  leak materially reduced (not eliminated)
- New AST-shape features ranked high in single-feature AUC:
  `max_nesting_depth` 0.93, `n_returns` 0.90 — flagged as possible per-corpus
  shape memorization, to be tested under LOO
- Conservative XGBoost CV AUC on faker-js alone: **0.986**
- **Residual catch in-corpus: 2/5** (`runtime_fetch_1`, `runtime_fetch_3`)

Pilot recommendation was PROCEED WITH CAVEATS — scale Fix A to other 5
corpora and run pooled + LOO before drawing conclusions.

Memo: [`era14-fixA-pilot.md`](era14-fixA-pilot.md)

### Phase 5 — Fix A full analysis (pooled + LOO across all 6 corpora)

After all 115 fixtures got hosts and re-extraction completed:

| Metric | Result | Pre-reg gate | Pass |
|---|---|---|---|
| Max single-feature AUC pooled | 0.886 (`n_unattested_callees` — honest Stage-3 output) | ≤ 0.85 | ✗ narrow miss |
| `hunk_file_callee_jaccard` AUC pooled | 0.703 (was 0.844 pre-fix) | ≤ 0.65 | close |
| Pooled conservative Set B 5-fold CV AUC | 0.9035 | ≥ 0.85 | ✓ |
| LOO test AUC ≥ 0.75 | **6 of 6 corpora** pass (faker-js weakest at 0.766) | ≥ 4 of 6 | ✓ |
| **Residual faker-js catch under LOO** | **0/5** at FP ≤ 0.9% | ≥ 2 of 5 | **✗** |

**Verdict: CLOSE NEGATIVE** on the residual-catch criterion (the era's primary
motivation). Aggregate generalization is excellent; the residuals are
structurally beyond what engineered-features-on-existing-stages can capture.

**Why the residuals collapsed under LOO**: top features in the held-out-faker-js
LOO model are existing-stage outputs:

1. `n_unattested_callees` (gain 22.06)
2. `bpe_score` (6.06)
3. `adjusted_bpe` (5.01)

The model is essentially re-learning Stage 3. But **the 5 residual fixtures
have `n_unattested_callees = 0` by definition** — that's exactly why era 11
misses them. A model whose dominant signal is "how many unattested callees"
cannot catch hunks where that count is 0.

The pilot's 2/5 in-corpus catch was driven by `max_nesting_depth` and
`n_returns` — features that DROPPED massively under LOO (0.93 in faker-js
alone → 0.69 pooled). That was per-corpus shape memorization, not
transferable signal. Fix A correctly exposed this by removing the routing
leak that had previously masked it.

Memo: [`era14-fixA-full.md`](era14-fixA-full.md)

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

## Pre-registered gates: where we stand (after all phases — final)

The complete trajectory across all four major approaches:

| Phase | Approach | Residuals caught (LOO + FP ≤ 0.9%) | Per-corpus FP | Status |
|---|---|---|---|---|
| 3-3.5 | Engineered XGBoost (leaky) | 5/5 (illusion) | n/a | Routing leak — closed |
| 5 | Engineered XGBoost (Fix A, leak-free) | 0/5 | within budget | Structural — `n_unattested_callees=0` for residuals |
| 6.3 | Embeddings supervised (LR/MLP/kNN) | 0/5 | various | Learned catalog-vs-real, not anomaly |
| 6.4 | Embeddings unsupervised (cluster centroid) | **1/5** (`runtime_fetch_2`) | within budget | **Best honest result** |
| 6.4b | Corpus-wide centroid fallback | 0/5 (regressed) | within budget | Threshold inflation killed the 1/5 |
| 7 | Per-cluster Mahalanobis on PCA-64 whitened (+ corpus-fallback) | 4/5 mechanically; **0/5 honest** | within budget | Rank-deficiency artifact — LOO controls also "catch" |
| 7.1 | PCA-whitened Euclidean to per-cluster μ_c (corpus-pooled implicit Σ) | **0/5** | within budget | Clean LOO (max ratio 1.56×). Residuals genuinely typical at top 67–80% of fjs controls |

**Final SHIP gate (≥2/5 residuals at FP ≤ 0.9% under LOO): not met by any phase.**

The 5 residual fixtures (`error_flip_2/3`, `runtime_fetch_1/2/3`) are
structurally beyond what any feature/model combination tested can capture
within the era-11 FP budget.

---

## Why the era didn't ship (yet)

The Phase 1-3.6b story was that bench data was designed for rule-based
scoring; ML training had two layers of leak (routing + fixture shape). Both
have now been addressed. After Fix A:

- The data is honest (LOO 6/6 corpora pass).
- The model genuinely learns existing-stage outputs (top feature
  `n_unattested_callees` with 4× the gain of the runner-up).
- But the residual fixtures have `n_unattested_callees = 0` by construction.
  No shallow model can learn signal that isn't in the input features.

**The structural conclusion**: catching the 5 residual faker-js fixtures
(`error_flip_2/3`, `runtime_fetch_1/2/3`) requires a NEW signal class that
existing stages don't emit. This is not a tuning problem; it's a feature-set
problem.

Candidates for a new signal class (each would be a future phase):

1. **Frozen pretrained code embedding** (e.g. UnixCoder — already a project
   dep for BPE tokenization). The encoder produces 768-dim semantic
   embeddings of code. A small head trained on (hunk_embedding,
   file_embedding) pairs could capture "semantic anomaly" — exactly the
   kind of signal needed for `Math.random in a person-name provider` even
   when `Math.random` is attested in tests. Era-1's pretrained encoder
   attempt (CodeRankEmbed at AUC 0.55) used pooled embeddings on raw code
   with a different objective; UnixCoder + contrastive head with our
   leak-fixed data structure has a real chance of working. **No GPU strictly
   needed for inference** (~30ms per hunk on CPU) — could be useful for
   training if we extend.

2. **Import-source aware features**. Features encoding "is this callee's
   defining module imported by this file?" For `Math.random` in a person
   provider: Math is a JS global, not imported. The feature would compare
   the file's import set to a curated map of "where each global typically
   appears." Limited applicability for browser globals; per-language
   heuristics needed.

3. **Sub-hunk embedding KNN to cluster-attested hunks**. For each cluster,
   store all attested hunks' embeddings. At score time, compute distance
   from the new hunk's embedding to its cluster's nearest neighbors.
   Anomaly = "this hunk doesn't look like anything in its cluster."

Option 1 (frozen UnixCoder + small head) is the highest-EV next phase —
direct semantic understanding, leverages existing dep, well-bounded
compute, addresses the structural diagnosis directly.

---

## What's been built (Fix A + Phase 6 + Phase 7 + Phase 7.1) vs what could come next

✓ **All six approaches in the obvious design space are tested.** Routing
leak fixed. Fixture-shape leak materially reduced. Engineered features,
supervised classifier on embeddings, unsupervised cosine-centroid scoring,
corpus-wide cosine-centroid fallback, PCA-whitened per-cluster Mahalanobis
distance, and the clean ablation (PCA-whitened Euclidean to per-cluster
μ_c) all measured. The honest signal exists in the embedding space
(Phase 6.2 cluster-centroid distance has AUC 0.91 on faker-js residuals
as a single feature) but it doesn't separate from the control distribution
at strict FP budgets — neither in raw cosine space (Phase 6.4: 1/5) nor
in PCA-whitened Euclidean space (Phase 7.1: 0/5). Covariance-based metrics
on small clusters fail a LOO sanity check (Phase 7).

**Why each approach didn't ship**:

1. **Engineered XGBoost** (Phase 5): the model's dominant input
   `n_unattested_callees = 0` for the residuals by definition (that's why
   era 11 misses them). No signal in the input features.

2. **Supervised embeddings** (Phase 6.3): pooled AUC 0.999 was
   catalog-detection learning. Under LOO, the model doesn't recognize
   held-out faker-js's catalog patterns because break categories vary
   across corpora.

3. **Unsupervised cosine centroid** (Phase 6.4): catches 1/5 (`runtime_fetch_2`).
   The other residuals sit too close to controls in cosine space at the
   strict FP budget. `error_flip_3` is genuinely typical (64th percentile);
   `error_flip_2`'s cluster has only 2 controls (statistically excluded).

4. **Corpus-wide cosine fallback** (Phase 6.4b): adds 173 high-distance
   unmappable controls to faker-js's calibration tail, raising the
   threshold past `runtime_fetch_2`'s distance. Strictly worse than 6.4.

5. **PCA-whitened per-cluster Mahalanobis** (Phase 7): mechanically catches
   4/5 residuals at SHIP-gate-passing FP — but a leave-one-out diagnostic
   shows 37 of 38 cluster covariance models are rank-deficient (30 have
   n_ctrl < PCA_dim = 64). After Tikhonov λI regularization, the inverse
   acts as 1/λ = 100 along the (64 − rank Σ) null-space directions, so
   any held-out point — break or control — gets an inflated d². LOO max
   control d² runs 5×–2500× the in-sample max. The "break detection"
   signal cannot be distinguished from inflation that would also flag
   held-out controls. Closes the covariance-based-cluster-metric axis.

6. **PCA-whitened Euclidean to per-cluster μ_c** (Phase 7.1): the clean
   ablation. Replaces broken Σ_cluster with corpus-pooled implicit Σ
   (via PCA whitening, n=297 ≫ d=64). LOO sanity passes cleanly: 0 of
   38 clusters flagged, max ratio 1.56× ≈ (k/(k-1))² for k=5 (the
   theoretical small-sample mean shift). The metric is honest and the
   honest answer is **0/5 residuals catch** — they sit at the 67th–80th
   percentile of fjs controls, genuinely typical-looking. The 36×–88×
   drop from Phase 7 d² to Phase 7.1 d² for the same fixtures is
   exactly the rank-deficient inflation, removed. PCA-whitening on
   its own merits is *worse* than Phase 6.4 cosine on residuals
   (0/5 vs 1/5). Closes the embedding-anomaly axis honestly.

**Future approaches that have NOT been tested in this era** (outside the
obvious design space, more invasive):

- **Encoder fine-tuning**: train UnixCoder itself with a contrastive
  objective on (cluster-positive, cluster-negative) pairs. Brings back
  full GPU/training complexity that era 1 stumbled on. Would need a much
  larger labeled set than 115 fixtures — synthetic mutation generation +
  contrastive pre-training, weeks of work.

- **Sub-hunk attention / token-level scoring**: instead of pooled [CLS]
  embeddings, score each token's anomaly via attention pooling against
  the file's typical tokens. Could surface "this specific call_expression
  is the anomaly inside an otherwise-normal hunk."

- **Multi-stage scoring with cross-feature learning**: jointly learn
  thresholds across (BPE, call-receiver, embedding-distance) at calibration
  time rather than per-stage. Would require redesigning the calibration
  pipeline.

- **Larger encoder** (CodeBERTa, CodeT5+): trade-off encoder size vs
  semantic discrimination. Untested whether a 2-3x larger encoder would
  spread `error_flip_3`'s embedding far enough from controls.

None of these are quick. Each is roughly an era's worth of research work.

**Operational decision**: Era 14 is closed on the embedding-anomaly
axis. Phase 7.1's clean ablation — PCA-whitening + per-cluster
anchoring with no per-cluster covariance — confirmed there is no
hidden signal that Phase 6.4's cosine metric was missing. The
residuals are typical-looking under any honest covariance-respecting
metric. The infrastructure on this branch is reusable. Production
scorer is unchanged at the era-11 baseline. The branch is preserved
as a research record (no PR opened). If a future approach in the
unexplored space — encoder fine-tuning, sub-hunk attention, larger
encoders, or a fundamentally new signal class — emerges as promising,
this branch is the launching point.

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
- Phase 4 (Fix-A pilot): [`era14-fixA-pilot.md`](era14-fixA-pilot.md)
- Phase 5 (Fix-A full + LOO): [`era14-fixA-full.md`](era14-fixA-full.md)
- Phase 6.2 (embedding probe): [`era14-phase6.2-embedding-probe.md`](era14-phase6.2-embedding-probe.md)
- Phase 6.3 (supervised embeddings + LOO): [`era14-phase6.3-loo.md`](era14-phase6.3-loo.md)
- Phase 6.4 (unsupervised centroid scoring): [`era14-phase6.4-centroid-anomaly.md`](era14-phase6.4-centroid-anomaly.md)
- Phase 6.4b (corpus-wide fallback — regressed): [`era14-phase6.4b-centroid-fallback.md`](era14-phase6.4b-centroid-fallback.md)
- Phase 7 (Mahalanobis on PCA-64 — rank-deficiency artifact): [`era14-phase7-mahalanobis.md`](era14-phase7-mahalanobis.md)
- Phase 7.1 (PCA-whitened Euclidean to cluster μ — clean ablation, 0/5): [`era14-phase71-whitened-euclidean.md`](era14-phase71-whitened-euclidean.md)
- Branch: `feat/era-14-ml-stage`, commits `8f52eb9` (extractor), `b74aed7` (routing+RAM fix), `2c8dcc4` (streaming sample), `2cb3e27` (host injection), `0052ff0` (UnixCoder embedder Phase 6.1), `7225cd4` (HF cache-first loading), `e153e9b` (Phase 6 close memo), `5a8c8f2` (Phase 7 close)
- Era-11 baseline (the production scorer this would have augmented): [`../11-cluster-conditional-attestation.md`](../11-cluster-conditional-attestation.md)

## End of Document
