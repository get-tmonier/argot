# Era 14 — ML Stage: Status & Findings to Date

> **Status**: PAUSED, NOT CLOSED. Era 14 is open to further phases — the
> engineered-features-on-existing-stages approach is structurally exhausted
> (Phase 5 below documents the LOO 0/5 residual catch). A semantic-embedding
> phase would be the natural next attempt. The current branch
> (`feat/era-14-ml-stage`) carries the leak-fixed feature extractor, host-file
> injection for all 115 fixtures, streaming RAM-bounded extraction, and saved
> models — useful infrastructure for any future ML attempt. Production scorer
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

## Pre-registered gates: where we stand (after Phase 5 / Fix A full)

| Gate | Threshold | Result | Pass |
|---|---|---|---|
| 1 (Phase 2) | Any feature AUC > 0.55 pooled | best 0.886 (`n_unattested_callees` — honest Stage-3 output) | ✓ |
| 2 (Phase 3 → Phase 5) | Pooled CV AUC > 0.85 | 0.972 (full), 0.903 (conservative Set B) | ✓ |
| 3 (Phase 5) | LOO held-out AUC ≥ 0.75 on ≥4 of 6 splits | **6 of 6** | ✓ |
| 4 (Phase 5) | ≥2 of 5 residual faker-js fixtures caught at faker-js FP ≤ 0.9% under LOO | **0/5** | ✗ |
| 5 (Phase 5) | Max single-feature AUC ≤ 0.85 (leak-free) | 0.886 | ✗ narrow (honest output, not leak proxy) |

Aggregate generalization passes (LOO 6/6 — the model genuinely transfers
across corpora). The era's motivating fixture-catch criterion fails: the
ML stage cannot catch what its dominant input feature (`n_unattested_callees`)
is structurally blind to.

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

## What's already been done (Fix A) vs what's next

✓ **Fix A is implemented and validated.** All 115 fixtures have `host_file`
+ `host_inject_at_line`. Routing leak gone. Fixture-shape leak materially
reduced. Data is honest (LOO 6/6 passes). Model trained on engineered
features generalizes well in aggregate (Set B AUC 0.90), but the residual
fixtures need a different signal class — see Phase 6 below.

**Phase 6 — Frozen UnixCoder embedding + small head (proposed next phase)**

The pre-registered structure mirrors Phases 1-5 with kill-switches:

- 6.1: extend the feature extractor to compute a 768-dim UnixCoder embedding
  per hunk (and per file context). One-time extractor change. UnixCoder
  encoder load adds ~500MB RAM per subprocess (still bounded).
- 6.2: per-dimension AUC pooled — at least one embedding dimension or
  small linear combination should reach AUC > 0.65 on the residual subset.
  Kill if no signal.
- 6.3: train a small MLP head (e.g. 256-dim hidden, 2 layers) on the
  embeddings + existing engineered features. Pre-reg gate: pooled CV AUC ≥
  existing 0.90 (must not regress).
- 6.4: LOO + residual catch under LOO. Pre-reg gate: ≥2 of 5 residual
  faker-js fixtures catch when faker-js is held out.
- 6.5: ship if all gates pass.

Compute estimate: ~30ms per hunk for UnixCoder forward pass on CPU × 1900
hunks × 6 corpora ≈ 6 minutes total extraction (still parallelizable with
the existing subprocess-per-corpus design). MLP training is fast. GPU not
strictly needed for inference; could speed up an extension that fine-tunes
the encoder or trains contrastive pairs at scale.

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
- Branch: `feat/era-14-ml-stage`, commits `8f52eb9` (extractor), `b74aed7` (routing+RAM fix), `2c8dcc4` (streaming sample), `2cb3e27` (host injection)
- Era-11 baseline (the production scorer this would have augmented): [`../11-cluster-conditional-attestation.md`](../11-cluster-conditional-attestation.md)

## End of Document
