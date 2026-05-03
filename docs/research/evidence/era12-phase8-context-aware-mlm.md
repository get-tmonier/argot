# Era 12 Phase 8 — Context-aware frozen-encoder anomaly: six variants, one ceiling

**Date**: 2026-05-03
**Branch**: `feat/era-12-ml-stage`
**Scripts**:
- `engine/scripts/era12_phase8_mlm_surprise.py` — Phase 8 (no context) and Phase 8.1 (with context, `--use-context` default)
- `engine/scripts/era12_phase8c_context_centroid.py` — Phase 8c
- `engine/scripts/era12_phase8d_hunk_context_divergence.py` — Phase 8d
- `engine/scripts/era12_phase8e_token_nn_in_context.py` — Phase 8e
- `engine/scripts/era12_phase8f_ensemble_maxz.py` — Phase 8f

---

## TL;DR

After Phase 7.1 closed the *covariance-based* metric family, Phase 8 attacked
the same problem from a different angle: instead of pooled embedding distance,
extract anomaly via per-token mechanisms. Six concrete variants tested.

| Variant | Mechanism | Production cost | Pooled AUC | fjs residual catch |
|---|---|---:|---:|---:|
| Phase 8 (no context) | per-token MLM surprise on hunk alone | 30+ fwd/hunk | 0.52 | 0/5 |
| **Phase 8.1** (with context) | per-token MLM with file-context | 30+ fwd/hunk | **0.42** (inverted) on 80-row smoke | 0/5 |
| Phase 8c | cluster-centroid cosine on `context_embedding` | 0 (precomputed) | — | 0/5 |
| Phase 8d | `cos(hunk_emb, context_emb)` per row | 0 | 0.57 | 0/5 |
| Phase 8e | per-token NN to context, single fwd | ~50ms/hunk | 0.70 | 0/5 |
| Phase 8f | MAX-z ensemble of (6.4 + 8d) | 0 | 0.68 | 0/5 |

**Phase 6.4's 1/5 (cluster-centroid cosine on `hunk_embedding`) remains the
ceiling on faker-js residuals.** Adding context (in any of the four ways
tested) plus ensembling does not exceed it.

---

## Phase 8 — per-token MLM surprise (era 12 redo, no context)

Era 12's MLM-surprise bakeoff used joint-masking under MPS memory pressure
(all hunk tokens masked at once) and got AUC 0.43, flagged in the memo as
likely confounded. Phase 8 redid it correctly: one mask at a time, batched in
groups of 8, CPU + fp32, hard-stop watchdog at 6 GB RSS.

**With `// Break:` meta-comments included in catalog hunk text:** AUC 0.79
on `surprise_mean`. Looks great.

**With meta-comments stripped (the actual code):** AUC drops to **0.52** —
random. The original signal was the meta-commentary describing the break
("// Break: Math.random instead of faker's internal RNG."), not the code.
Per-token diagnostic confirmed: top-5 surprise tokens for every residual
are newlines (`Ċ`, `ĊĊ`), not the actual anomalous content (`Math`, `fetch`).

Era-12 confound check: `surprise_mean` AUC went from 0.43 (joint-mask) to
0.52 (per-token, comments-stripped). The *direction* improved (no longer
inverted), confirming joint-masking was a real confound, but the proper
measurement still gives random on the actual code.

**Architectural diagnosis:** scoring the hunk in isolation cannot detect
contextual anomalies. `Math.random()` in a 50-token snippet is normal
JavaScript code; only against the surrounding faker-js file is it anomalous.

---

## Phase 8.1 — per-token MLM with file context (the architectural fix)

Same per-token MLM, but tokenize `[file_before, hunk, file_after]`
(post-injection for breaks via `synthesize_hunk_in_host`), window 510 tokens
centred on the hunk, mask only hunk-positions one at a time.

**Initial 30-row smoke (faker-js): surprise_mean AUC 0.65 (+0.13 over no-context).**
`runtime_fetch_2` rank moved from top 35% to top 17%.

**Full 80-row smoke (17 breaks + 63 controls): AUC 0.42 (inverted).**
`runtime_fetch_2` rank: top 45%. Same locale-data-tail problem that killed
Phase 6.4b — `src/locales/...` files have rare locale strings that score
high per-token surprise even with file context, dominating the threshold
quantile. The 30-row smoke was a sample-size artifact: 23 controls happened
to be less anomalous than the residuals.

**Production cost:** 30+ forward passes per hunk on 512-token sequences.
Smoke run took 30+ seconds per row on CPU. Unshippable as a stage 4.

---

## Phase 8c — cluster-centroid cosine on `context_embedding` (free)

Same logic as Phase 6.4 but use the precomputed `context_embedding`
(768-d, 512-token window centred on the hunk) instead of `hunk_embedding`.
Asks "is the *file context* of this hunk anomalous in cluster?"

**Result: 0/5 residuals catch.** Per-corpus FP regression on faker-js
(1.65% vs target 0.9%) means even calibration fails. The `context_embedding`
includes the hunk content (the window is centred on it), and it gets averaged
with the surrounding 460 tokens of host file content. The break's specific
contribution is smeared into a typical-looking aggregate.

`runtime_fetch_2` rank: top 5.0% (vs Phase 6.4's top 0.83%) — strictly
worse. Phase 6.4's hunk-only view was sharper.

---

## Phase 8d — `cos(hunk_emb, context_emb)` per row (free)

Asks: "how well does this hunk align with its own file context?" For a
normal control hunk, hunk and context are semantically aligned; for a
break that introduces foreign semantics, they should diverge.

**Result: 0/5 fjs residuals, but high per-corpus AUC on other corpora:**

| Corpus | AUC | Recall (at 0.5pp budget) |
|---|---:|---|
| ink | **0.86** | 7/17 (41.2%) |
| hono | **0.83** | 0/17 |
| faker-js | 0.80 | 0/17 |
| faker | 0.61 | 1/16 |
| rich | 0.52 | 0/16 |
| fastapi | **0.18 (inverted)** | 0/32 |

The metric works strongly on **ink** (small, semantically-cohesive library)
but inverts on **fastapi** (route handlers naturally diverge from their
methods). It misses the faker-js residuals because their host files are
semantically aligned with the break content (image-provider host + image-
fetch break → small divergence).

---

## Phase 8e — per-token NN distance, single forward pass (cheap, production-viable)

The cheap version of Phase 8.1's "per-token + context" insight: ONE forward
pass on `[hunk + 460-token context]`, take last hidden states, for each hunk
token compute cosine distance to its NEAREST context token, aggregate per
hunk.

**Result: 0/5 fjs residuals.** Per-corpus AUC pattern is the *opposite* of
8d's:

| Corpus | 8e (token-NN max) | 8d (hunk-context cos) |
|---|---:|---:|
| fastapi | **0.84** | 0.18 |
| rich | **0.79** | 0.52 |
| faker-js | 0.41 | 0.80 |
| hono | 0.38 | 0.83 |
| ink | 0.44 | 0.86 |

So 8d and 8e capture orthogonal structure across corpora. But neither catches
faker-js residuals.

**Per-token diagnostic does surface real semantic content tokens** for the
fjs residuals — `runtime_fetch_1`'s top-5 includes `·fetch`,
`runtime_fetch_2`'s includes URL syntax (`://`, `picsum/{...}`),
`runtime_fetch_3`'s includes `aw(ait)` and `://`, `error_flip_3`'s includes
`·throw`, `·requires`, `·undefined`. The encoder *can* see the semantically
anomalous tokens. But at NN distance ~0.23–0.31 vs faker-js threshold ~0.60,
the magnitude is only 38–52% of the way to flagging. The host files (image,
company, locale modules) have neighbouring tokens (`url`, `Promise`, `async`)
that reduce the NN distance to fetch.

**Production cost:** 5.74 rows/sec on CPU. ~50ms/hunk at score time
(versus 30s for MLM). Shippable.

---

## Phase 8f — MAX-z ensemble of (6.4 + 8d) (free)

For each row: z-normalise Phase 6.4's centroid distance and Phase 8d's
hunk-context cosine against per-corpus controls, take MAX. Threshold
calibrated per-corpus on the max-z control distribution.

**Result: 0/5.** `runtime_fetch_2` misses by **0.02 z-score** (z=2.385 vs
threshold 2.406). The ensemble actually *demoted* RT_2 from top 0.83% (under
6.4 alone) to top 1.0% — Phase 8d's noise lifted the threshold.

Pooled AUC: 0.68 (worse than 6.4 alone at 0.73). The ensemble traded
specificity for breadth.

| Residual | z_64 | z_8d | max_z | top-X% (fjs) |
|---|---:|---:|---:|---:|
| error_flip_2 | — (excluded) | 0.91 | 0.91 | 22.8% |
| error_flip_3 | 0.39 | 0.78 | 0.78 | 27.2% |
| runtime_fetch_1 | 2.09 | 1.39 | 2.09 | 2.0% |
| runtime_fetch_2 | 2.39 | 0.69 | 2.39 | 1.0% |
| runtime_fetch_3 | 1.63 | 0.64 | 1.63 | 6.4% |

Side win: ink corpus catches 4/17 (23.5%) at +0.19pp FP — better than 6.4
alone (3/17, 17.65%). Different corpus, different mechanism — context
alignment helps where the corpus is small and semantically tight.

---

## Architectural lessons across the six variants

1. **Per-token preserves anomaly; pooled-embedding averages it away.** Phase 8
   no-context (AUC 0.52 random) vs Phase 8.1 with-context smoke 30-row
   (AUC 0.65) showed the architectural fix matters — *until* Phase 8.1's
   80-row run revealed the locale-data-tail problem.

2. **The locale-data tail is the binding constraint on faker-js.** 173 of 298
   controls are `src/locales/...` files with rare per-locale strings. Any
   per-corpus metric (cosine, Mahalanobis, MLM surprise, per-token NN)
   has its 99-percentile dominated by these. The residuals' anomaly magnitude
   doesn't exceed the locale tail under any encoder-based metric tested.

3. **Different per-corpus structures need different metrics.** 8d (hunk-vs-
   context cosine) is great on ink/hono/fjs (AUCs 0.80–0.86). 8e (per-token
   NN) is great on fastapi/rich (AUCs 0.79–0.84). Neither is great everywhere.
   This suggests no single frozen-encoder metric will cover all six corpora.

4. **The faker-js residuals are NOT semantic-embedding anomalies.** Their
   host files are semantically close to the break content (image-provider
   host + image-network break, person-name host + person-name break). The
   anomaly is **lexical / import-graph** — `fetch` is a JS global never
   imported in faker-js, `Math.random` is forbidden by project policy that
   no embedding-based detector can read.

5. **Era 12's "MLM cleanly ruled out" verdict was confounded.** Joint-masking
   on hunk tokens was the issue. Per-token masking with comments stripped
   shows MLM is essentially random on the code itself (architectural
   limitation, not a tooling artifact). With file context, signal *appears*
   on small samples but disappears on full-corpus calibration due to the
   locale-data tail.

---

## Where the next experiment should focus

**Phase 9 — import-source-aware rule-based features.** The diagnosed
mechanism: `fetch` is a JS global never imported by faker-js providers.
`Math.random` is a JS global the project policy forbids. `crypto.randomBytes`
requires `import { randomBytes } from 'crypto'` which faker-js does not have
in its provider modules. These are **structural** anomalies any rule-based
detector can find. They are invisible to embedding metrics because the
encoder doesn't know about project-level import policies.

A minimum viable Phase 9: for each hunk, count tokens that are
- JS globals from a hard-coded list (`fetch`, `XMLHttpRequest`, `crypto`,
  `process`, ...)
- AND not imported in the host file's import set

Calibrate per-corpus FP threshold; apply to the 5 fjs residuals. If
≥ 2/5 catch at era-11 FP budget, Phase 9 ships and era 12 closes positive.

The honest expected value is high (60–80%) on `runtime_fetch_*` since all
three use `fetch`. `error_flip_*` use `throw new Error(...)` which is also
a global pattern but more common across JS code → lower expected catch on
those two. So 3/5 is a realistic optimistic target.

Era 12 stays open until Phase 9 is tested.
