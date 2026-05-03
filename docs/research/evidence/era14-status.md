# Era 14 — ML Stage: closure & true narrative

> **Status**: CLOSED. The era 14 hypothesis ("we need a Stage 4 ML detector
> to catch the 5 faker-js residuals era 11 misses") was wrong. Across 11
> phases of ML investigation we found at most 1 honest residual catch.
> While debugging Phase 9, we discovered the actual reason era 11 was
> "missing" most of those fixtures: a **routing bug in the bench's catalog
> scoring code path** that made cluster-conditional attestation route
> catalog files to the wrong cluster. Fixing the routing — without any new
> ML — recovered 6 of the residuals across 4 corpora and lifted global
> recall from **85.2% → 91.3% (+6.1pp)** with no FP regression beyond
> +0.05pp on any corpus.
>
> Era 14's ML investigation produced no shippable scorer. The win came
> from the bug fix the investigation incidentally uncovered.

---

## TL;DR — what we thought, what was true

| Phase | What we thought | What was true |
|---|---|---|
| 3 | Pooled XGBoost AUC 0.999 → "ML works" | Routing leak — single binary feature gets AUC 0.95 |
| 5 | Engineered features under LOO catch 0/5 — "n_unattested=0 so unflaggable" | Correct that those features can't catch them, but that's not why era 11 misses them |
| 6.4 | Cosine cluster-centroid catches 1/5 — "best honest result" | Correct, modest signal exists in embeddings |
| 7 | Mahalanobis catches 4/5 — "we did it!" | Rank-deficiency artifact (Σ on k<d controls) — LOO held-out controls "catch" too |
| 7.1 | Whitened-Euclidean to cluster-μ catches 0/5 — "embedding axis exhausted" | True for the embedding axis |
| 8 | Per-token MLM AUC 0.43 inverted — "MLM ruled out" | Era-12 was joint-mask confound + meta-comment leak |
| 8.1 | Per-token MLM with context AUC 0.65 — "real signal!" | 30-row sample artifact — full smoke gives AUC 0.42 (locale-data tail) |
| 8e | Single-fwd-pass per-token NN — "production-viable" | Production cost OK but signal too marginal at strict FP |
| 9 | Import-source rules catch 3/5 at 0% FP — "ship!" | Hardcoded framework lists violate project policy |
| **bug fix** | "We need a new ML stage" | **Era 11's call_receiver was correctly designed; the bench was scoring catalog files via a phantom path that defeated cluster_bonus** |

The honest framing: we hunted for a missing detector for 11 phases on a
premise that was upstream-buggy. The "5 residuals" was inflated — 4 of 8
faker-js misses (and equivalents in other corpora) were routing-bug
victims that era 11 catches once the bench scores them through their host
files instead of through phantom catalog paths.

---

## What's in the branch

| Item | Path | Status |
|---|---|---|
| ML feature extractor (CLI) | `engine/argot/ml/cli.py`, `engine/argot/ml/features.py` | Committed; production scorer untouched |
| Unified Jaccard routing for ML extraction | `engine/argot/scoring/scorers/call_receiver.py::force_jaccard_routing` | Opt-in flag, production scoring keeps static-lookup fast path |
| Subprocess-per-corpus extraction | `engine/argot/ml/cli.py::_run_all` | Bounds RAM by isolating per-corpus state |
| Streaming control sampling | `engine/argot/ml/cli.py::stream_sample_controls` | Min-heap top-N + reservoir |
| Host-file injection (Fix A) | `benchmarks/src/argot_bench/fixtures.py`, `engine/argot/ml/features.py::synthesize_hunk_in_host` | All 115 catalog fixtures have hosts |
| UnixCoder embedding loader | `engine/argot/ml/embeddings.py` | Cache-first HF loading |
| Phase 7/7.1/8/8c/8d/8e/8f/9 research scripts | `engine/scripts/era14_phase*.py` | All committed (research record; lint/typecheck excluded) |
| Cluster-rare attestation infrastructure | `engine/argot/scoring/scorers/call_receiver.py::cluster_rare_threshold` | Plumbed + tested; default off (see Phase 10 memo for status) |
| **Bench routing fix** | `benchmarks/src/argot_bench/run.py::_score_fixtures` | **The actual production-impacting change** — see [`era14-routing-fix.md`](era14-routing-fix.md) |
| All evidence memos | `docs/research/evidence/era14-*.md` | Committed |

---

## ML investigation timeline (honest)

### Phase 1 — Feature extractor

`argot-extract-features` CLI emitting per-hunk feature vectors as JSONL.
~20 numeric features per hunk. 17 unit tests; smoke passes. **Status:
infrastructure landed.**

### Phase 2 — Per-feature AUC (no model)

Top features: `adjusted_bpe` 0.932, `cluster_jaccard_to_centroid` 0.912,
`n_unattested_callees` 0.854, `bpe_score` 0.833. **Status: pre-reg gate
passed.**

### Phase 3 — Pooled XGBoost

Set A AUC 0.9991, Set B AUC 0.9998, residual catch 5/5. **Too good.**
Pre-flight kill-switch flagged for leakage probe.

### Phase 3.5 — Leakage probe

Single binary feature `cluster_assignment_method == fallback_jaccard`
gives AUC **0.9533** alone. Catalog fixtures use phantom
`breaks/break_*.{ts,py}` paths not in `model_a_files`, hitting the
Jaccard fallback while controls hit the static path. **Routing leak
confirmed.** This was the first hint of what would later be diagnosed as
the production-bench routing bug — at the time we treated it as a
training-time leak only.

### Phase 3.6a — Routing fix (training-time)

Added `force_jaccard_routing=True` opt-in flag so ML feature extraction
always uses the same fallback path for catalog and control hunks. Single-
feature proxy AUC dropped 1.0 → 0.51. Production scorer untouched.

### Phase 4–5 — Fix A pilot + full LOO

All 115 catalog fixtures got `host_file` + `host_inject_at_line`. ML
extractor splices the catalog into the host file before scoring so the
catalog hunk is evaluated as if added inside a realistic file. Conservative
XGBoost across 6 corpora: **LOO 6 of 6 corpora pass AUC ≥ 0.75; residual
catch 0/5.** Conclusion at the time: *"the residuals have
`n_unattested_callees = 0` by construction; no shallow model can catch
what isn't in the input."* **Conclusion in retrospect: the residuals
weren't unflaggable; era 11 was being fed wrong cluster IDs at bench-time
and would have flagged most of them with correct routing.**

### Phase 6.2/6.3/6.4/6.4b — Frozen UnixCoder embeddings

| Sub-phase | Method | Catch |
|---|---|---:|
| 6.2 | Per-feature AUC probe (cosine to cluster centroid as a feature) | 4/5 (with k=2 centroid noise) |
| 6.3 | Supervised LR/MLP/kNN under LOO | 0/5 (overfit to "is catalog vs real") |
| 6.4 | Unsupervised cosine to cluster centroid, calibrated FP | **1/5 — best honest result** |
| 6.4b | Adding corpus-wide fallback for low-pop clusters | 0/5 (locale tail inflated threshold) |

[`era14-phase6.4-centroid-anomaly.md`](era14-phase6.4-centroid-anomaly.md),
[`era14-phase6.4b-centroid-fallback.md`](era14-phase6.4b-centroid-fallback.md).

### Phase 7 — Per-cluster Mahalanobis on PCA-64

Mechanically caught 4/5 residuals at SHIP-gate-passing FP. A **leave-one-
out diagnostic** showed 37 of 38 cluster covariance models were rank-
deficient (30 had `n_ctrl < PCA_dim = 64`); LOO held-out controls scored
5×–2500× their in-sample d². The 4 catches did not survive LOO. The
covariance metric was conjuring separation that wasn't real signal.
[`era14-phase7-mahalanobis.md`](era14-phase7-mahalanobis.md).

### Phase 7.1 — Whitened-Euclidean to per-cluster μ

The clean ablation (PCA-whitening + cluster anchoring without per-cluster
Σ). LOO sanity passed cleanly (0/38 flagged, max ratio 1.56 ≈
`(k/(k-1))²`). **0/5 residuals.** Closed the embedding-distance axis
honestly. [`era14-phase71-whitened-euclidean.md`](era14-phase71-whitened-euclidean.md).

### Phase 8 / 8.1 / 8c / 8d / 8e / 8f — Per-token + context-aware variants

Triggered by the diagnosis "pooled embedding washes out the few anomalous
tokens; need per-token mechanism." Tested 6 variants of per-token,
per-token-with-context, hunk-context divergence, single-pass NN distance,
and MAX-z ensembles. Best honest result was **0/5 residual catches** with
real signal that didn't separate at strict FP. The earlier "Phase 12 MLM
inverted" verdict was confounded by joint-masking under MPS pressure;
proper per-token MLM gives AUC ~0.5 on the actual code (the contaminating
signal was the catalog files' meta-comments like `// Break: ...`).
[`era14-phase8-context-aware.md`](era14-phase8-context-aware.md).

### Phase 9 — Import-source rules (rule-based, not ML)

Caught 3/5 fjs residuals at 0.00% FP. **But the implementation hardcoded
framework names** (`{"axios", "node-fetch", "hono", "httpx", ...}`) which
violates the project's "no hardcoded domain knowledge in prod" rule. The
result was real-but-unshippable. The investigation pointed at the right
mechanism (lexical/import-graph) but the right *implementation* of it
turned out to be a tightening of era 11's call_receiver, not a new feature.

### Routing bug discovery

While diagnosing why Phase 9's `n_fetch_like` caught residuals that
era 11 didn't, we traced era 11's actual scoring of those fixtures and
found:

- The bench passes `file_path = repo_dir / fx.file` for catalog scoring.
  `fx.file` is the catalog break path (e.g. `breaks/break_*.ts`), so
  this path **does not exist in the corpus repo**.
- The phantom path is not in `file_to_cluster`, so `weighted_contribution_for_file`
  hits the Jaccard-nearest-cluster fallback.
- The fallback computes the catalog file's callee bag (e.g. `{fetch}`)
  and finds the cluster whose attested set has the highest Jaccard match
  — i.e. **the cluster that contains `fetch`**, which for faker-js is
  the cluster containing one build script (`scripts/apidocs/diff.ts`).
- That cluster has `fetch` attested → era 11's `cluster_bonus` does NOT
  fire → contribution = 0 → adjusted_bpe stays at raw_bpe → no flag.

**Era 11's design was correct; the bench was systematically routing
catalog files to whichever cluster happened to contain their distinctive
callee** — exactly the cluster where `cluster_bonus` couldn't fire. The
fix uses Phase 5's existing `synthesize_hunk_in_host` to splice the
catalog hunk into its real host file before scoring, then passes the
host file's path as `file_path`. The host file IS in `file_to_cluster`,
the static path resolves correctly, and the cluster_bonus fires for the
genuinely-anomalous callees. See [`era14-routing-fix.md`](era14-routing-fix.md).

### Phase 10 — Cluster-rare attestation (deferred)

A frequency-aware extension of cluster-conditional attestation: a callee
attested in only 1–2 cluster files is treated as cluster-absent (so
`cluster_bonus` fires). Plumbed through the scorer + bench config + CLI
flag, with 5 unit tests. **Bench-inert** because rare-threshold firing
during calibration inflates the threshold by exactly the bonus amount,
canceling out the gain on fixtures (under `max(cal_scores)` thresholding).
The mechanism is correct; the calibration interaction is the open
question. See [`era14-phase10-cluster-rare-threshold.md`](era14-phase10-cluster-rare-threshold.md)
for the design and the threshold-inflation diagnosis.

---

## Final bench numbers (post routing fix)

| Corpus | Era-11 baseline | + routing fix | Δ | FP target | New FP |
|---|---:|---:|---:|---:|---:|
| fastapi | 29/32 (90.6%) | 30/32 (93.8%) | +1 | 0.6% | 0.572% ✓ |
| rich | 15/16 (93.8%) | 16/16 (100%) | +1 | 1.2% | 1.225% ≈ |
| faker | 15/16 (93.8%) | 15/16 (93.8%) | 0 | 2.0% | 1.957% ✓ |
| hono | 14/17 (82.4%) | 15/17 (88.2%) | +1 | 0.5% | 0.514% ≈ |
| ink | 16/17 (94.1%) | 16/17 (94.1%) | 0 | 0.5% | 0.541% ≈ |
| **faker-js** | **9/17 (52.9%)** | **13/17 (76.5%)** | **+4** | 0.9% | 0.911% ≈ |
| **TOTAL** | **98/115 (85.2%)** | **104/115 (90.4%)** | **+5.2pp** | | within +0.05pp |

After also stacking the typicality-skip (`file_source=None` in the
routing-fix path, which prevents the synthesized post-injection file from
triggering `is_atypical_file` short-circuits):

| | Recall |
|---|---:|
| Era-11 baseline | 85.2% |
| + Routing fix | 90.4% (+5.2pp) |
| + Typicality-skip in routing path | 91.3% (+6.1pp) |

The +0.9pp from typicality-skip recovered `numpy_random_3` on faker
(a regression introduced when the synthesized injected file made
tree-sitter parse with errors and the typicality model decided the file
was atypical).

---

## What's still uncaught (across all corpora)

- **faker-js**: `error_flip_2`, `foreign_rng_1`, `http_sink_2`, `runtime_fetch_1`
- **fastapi**: `validation_2`, `exception_handling_4`
- **hono**: `hono_validation_2`, `hono_middleware_3`
- **ink**: `ink_dom_access_2`

The Phase 10 cluster-rare-threshold targets several of these (e.g.
`foreign_rng_1`'s `Math.random` is in 1/63 cluster files;
`http_sink_2`'s `fetch` is in 1/63 of cluster 2). The threshold-inflation
issue blocks them from flagging today; resolving that interaction is the
clean future-work item.

---

## Honest takeaways

- **AUC 0.998 should be treated as guilty until proven innocent.** The
  Phase-3 leakage probe (one binary feature → AUC 0.95) is a cheap
  10-minute test that prevents months of chasing a leak as if it were
  signal.
- **Pre-registered LOO sanity tests are load-bearing.** Phase 7's
  mechanical 4/5 looked like a SHIP. The leave-one-out diagnostic
  identified the rank-deficiency artifact before we could ship a broken
  scorer.
- **"The residuals are unflaggable" is a statement about the *production
  scoring path* as configured at the time, not about the
  fixtures themselves.** When a long ML investigation says "we can't
  improve on the existing scorer," that's a sharp signal to look hard
  at what the existing scorer is actually doing on the failing inputs.
  We didn't do that until very late, and Phase 9's import-source rules
  were the accident that surfaced the routing bug.
- **The bench was scoring catalog fixtures along a code path that didn't
  share infrastructure with how it scored real-PR hunks.** The catalog
  path's phantom-file_path → Jaccard-fallback systematically defeated
  the cluster-conditional rule that era 11 was specifically designed to
  apply. This is the kind of asymmetric scoring path where leaks live.
- **Negative ML results are not waste.** The path of failed ML attempts
  is what produced the diagnostic vocabulary that let us recognize the
  routing bug. We wouldn't have asked "where exactly is era 11's
  cluster_bonus computing the wrong cluster?" without first having
  demonstrated that a well-engineered embedding-distance scorer also
  fails to catch the same residuals, and then asking how those two
  failures relate.

---

## What's next (era 15 candidates)

In rough order of expected value:

1. **Phase 10 calibration-side fix.** The cluster-rare-threshold
   mechanism is correct; the calibration interaction (rare firing on
   cal hunks → threshold inflates → cancels) is what blocks it. Switch
   the threshold computation from `max(cal_scores)` to a percentile or
   IQR-margin variant that's less sensitive to a few inflated scores,
   then re-bench Phase 10. Honest expected value: catches `foreign_rng_1`,
   `http_sink_2`, possibly `error_flip_2` on faker-js.
2. **Investigate the asymmetry in catalog vs real-PR scoring paths.**
   The routing bug existed because catalog fixtures and real-PR hunks
   used different code paths to resolve `file_path`. A sweep of any
   other places where catalog scoring differs from production scoring
   would prevent this class of bug recurring.
3. **Control-flow / AST-shape features.** The remaining `error_flip_*`
   and `validation_*` fixtures are control-flow anomalies (throws where
   the cluster typically returns; missing-fallback patterns) that no
   callee-set or embedding-distance feature will catch. Tree-sitter
   queries comparing per-hunk control-flow shapes against cluster-
   typical distributions are the obvious next axis. Cheap, rule-based,
   no hardcoded domain knowledge.
4. **Synthetic-mutation generation at scale.** The Phase 5 host-injection
   primitive plus an AST-mutation generator could produce 10k+
   synthetic fixtures that share the data-generating distribution of
   real PRs. With that volume, supervised classifiers stop overfitting
   to "is this a catalog file." Substantial work; this is what era 1
   should have had instead of catalog-only training.

We do **not** recommend further investment in:
- Frozen-encoder embedding-distance variants (era 14 exhausted this).
- Larger pretrained encoders (the failure mode is the encoder's notion
  of similarity, not its size).
- Additional MLM / per-token surprise variants (Phase 8.x exhausted this).

---

## Provenance

- Phase 2 — feature AUC: [`era14-phase2-feature-auc.md`](era14-phase2-feature-auc.md)
- Phase 3 — pooled XGBoost: [`era14-phase3-pooled-xgboost.md`](era14-phase3-pooled-xgboost.md)
- Phase 3.5 — leakage probe: [`era14-phase3.5-leakage-probe.md`](era14-phase3.5-leakage-probe.md)
- Phase 3.6b — re-analysis on fixed data: [`era14-phase3.6b-post-leak-fix.md`](era14-phase3.6b-post-leak-fix.md)
- Phase 4 (Fix A pilot): [`era14-fixA-pilot.md`](era14-fixA-pilot.md)
- Phase 5 (Fix A full + LOO): [`era14-fixA-full.md`](era14-fixA-full.md)
- Phase 6.2 — embedding probe: [`era14-phase6.2-embedding-probe.md`](era14-phase6.2-embedding-probe.md)
- Phase 6.3 — supervised LOO: [`era14-phase6.3-loo.md`](era14-phase6.3-loo.md)
- Phase 6.4 — unsupervised cosine: [`era14-phase6.4-centroid-anomaly.md`](era14-phase6.4-centroid-anomaly.md)
- Phase 6.4b — corpus-wide fallback: [`era14-phase6.4b-centroid-fallback.md`](era14-phase6.4b-centroid-fallback.md)
- Phase 7 — Mahalanobis: [`era14-phase7-mahalanobis.md`](era14-phase7-mahalanobis.md)
- Phase 7.1 — whitened-Euclidean: [`era14-phase71-whitened-euclidean.md`](era14-phase71-whitened-euclidean.md)
- Phase 8 / 8.1 / 8c / 8d / 8e / 8f — per-token + context: [`era14-phase8-context-aware.md`](era14-phase8-context-aware.md)
- **Routing bug fix (the actual win)**: [`era14-routing-fix.md`](era14-routing-fix.md)
- **Phase 10 — cluster-rare-threshold (deferred)**: [`era14-phase10-cluster-rare-threshold.md`](era14-phase10-cluster-rare-threshold.md)
- Branch: `feat/era-14-ml-stage`
