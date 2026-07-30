# P2 — pushing static embeddings past their ceiling

**Date:** 2026-07-29
**Plan:** `.scratch/plan-static-embedder.md`
**Prior:** `static-embedder-P1.md` established that every static model tested
lands at 0.70–0.73 of the heavy model's findings (fair matched-gate metric), and
that this is a ceiling of the representation class rather than a model-choice
problem.

**Question:** the direction is chosen — static, no llama.cpp. How much of the
lost 27% can be recovered by post-processing, without reintroducing the heavy
model at check time?

Metric throughout: each scorer uses **its own** gate, calibrated to fire on as
many queries as the heavy model does. Reported number = fraction of the heavy
model's findings that survive.

## What was tried

| lever | needs heavy model? | when |
| --- | --- | --- |
| all-but-the-top (ABTT) | no | fit, milliseconds |
| whitening | no | fit, milliseconds |
| model ensemble | no | fit, one extra encode per model |
| learned projection static→heavy | **yes**, N sample pairs | fit |

## Post-processing and alignment (MSEgui, held-out half)

| configuration | held-out |
| --- | ---: |
| jina-v2-code-static-256, raw | 0.7228 |
| + ABTT(8) | 0.7455 |
| + ridge projection | 0.7660 |
| **+ whitening then ridge projection** | **0.7772** |
| ABTT(8) then ridge projection | 0.7563 |
| ensemble of 4 statics, ABTT(8) each | 0.7479 |
| ensemble of 4 statics → ridge | 0.7618 |
| **ensemble of 4 statics → whitening + ridge** | **0.7897** |
| procrustes (orthogonal map) | 0.7033–0.7131 — useless |

- **Whitening before the alignment beats plain ridge**, consistently.
- **ABTT and the projection are redundant**: stacking them *loses* 2 points
  (0.7563 vs 0.7772). The projection already absorbs the anisotropy that ABTT
  corrects by hand.
- Procrustes' orthogonality constraint is too rigid to help at all.
- A 4-model ensemble adds ~1.3 points over the best single model, at the cost of
  loading four models — poor value for a "model embedded in the binary" design.

## How many pairs does the projection need?

| training pairs | held-out | heavy-embed cost (CPU / Metal) |
| ---: | ---: | --- |
| 0 (raw static) | 0.7228 | — |
| 200 | 0.7326 | 9 s / 3 s |
| 500 | 0.7521 | 22 s / 7 s |
| **1,000** | **0.7730** | **43 s / 14 s** |
| 2,000 | 0.7716 | 87 s / 28 s |
| 5,000 | 0.7660 | 217 s / 69 s |
| 13,053 | 0.7772 | 568 s / 180 s |

**The curve saturates at ~1,000 pairs.** Beyond that the movement is noise (the
held-out set holds 718 firing queries, so ±1% is not a signal). A repo-specific
alignment therefore costs **43 seconds of heavy embedding on a CPU runner** —
against the 19 minutes a full heavy index costs.

## Does the projection transfer across repos? (the shipping question)

If a matrix trained on our corpora helps a repo it has never seen, one **generic**
matrix ships in the binary and the user pays nothing. Otherwise it must be
fitted per repo.

|  | msegui (pascal) | fastapi (python) | hono (typescript) |
| --- | ---: | ---: | ---: |
| no projection | 0.7186 | 0.9083 | 0.6512 |
| trained on msegui | **0.7921** | 0.9037 | 0.6395 |
| trained on fastapi | 0.7168 | **0.9358** | 0.6512 |
| trained on hono | 0.7419 | 0.9083 | **0.7209** |
| leave-one-out (generic) | 0.7473 (+0.029) | 0.9037 (−0.005) | 0.6628 (+0.012) |

- **On the diagonal the gain is large** (+0.074, +0.028, +0.070).
- **Off the diagonal it is neutral to negative.** The alignment is largely
  repo-specific.
- The leave-one-out "generic" case averages **+0.012** — small and inconsistent,
  including one regression.

**Caveat that limits this conclusion:** fastapi and hono contribute only 650 and
608 functions, so the leave-one-out matrix for MSEgui was fitted on ~1,258
examples for a 256×768 map (196k parameters). That is badly underpowered; a
genuine generic matrix would be trained on tens of thousands of functions across
many corpora. The negative transfer result is **suggestive, not settled**.

## The finding that matters most

The static penalty is **not a constant** — it depends heavily on the corpus:

| corpus | static-only, no post-processing |
| --- | ---: |
| fastapi (python) | **0.9083** |
| msegui (pascal) | 0.7186 |
| hono (typescript) | 0.6512 |

Static loses **9%** on FastAPI and **35%** on Hono. MSEgui — the repo that
started this whole investigation — is a hard case: Object Pascal with long
lowercase compound identifiers (`projectsaveexe`, `forcezorderexe`) that a
general tokenizer shreds, plus 74.5% of its functions having a near-duplicate.

This means a single headline number for "static costs you X%" is misleading, and
argot should *measure it per repo* rather than assume it — which is cheap to do
whenever a sample of heavy pairs exists.

## Design implication

Because the sample that trains the projection also **measures** the residual
quality (the heavy answer is known for those 1,000 functions), a fit that spends
43 seconds can report honestly: *"on this repo the static index recovers 77% of
the contextual model's duplicate findings"*. That is argot's own philosophy —
measure the guardrail's limits, do not assert them.

## Free quality: post-processing with no heavy model at all

Computed from the static vectors alone, at fit, in milliseconds.

| transform | msegui | fastapi | hono | saleor | mean | Δ |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 0.7186 | 0.9083 | 0.6512 | 0.6679 | 0.7365 | — |
| whitening | 0.7419 | 0.9037 | 0.6860 | 0.6272 | 0.7397 | +0.003 |
| ABTT(1) | **0.7608** | 0.9041 | 0.7011 | 0.6580 | 0.7560 | +0.020 |
| **ABTT(4)** | 0.7590 | 0.8899 | **0.7209** | 0.6654 | **0.7588** | **+0.022** |
| ABTT(8) | 0.7554 | 0.8853 | 0.6977 | 0.6531 | 0.7479 | +0.011 |
| ABTT(16) | 0.7572 | 0.8899 | 0.6552 | 0.6444 | 0.7367 | +0.000 |
| ABTT(32) | 0.7348 | 0.8904 | 0.6395 | 0.6235 | 0.7220 | −0.014 |
| whitening + ABTT(8) | 0.7366 | 0.8904 | 0.6552 | 0.6123 | 0.7236 | −0.013 |

- **ABTT(1–4) is worth ~+2 points for free**, and it helps most exactly where
  the baseline is worst (+4.0 on MSEgui, +7.0 on Hono) while costing ~1 point on
  the corpus that was already good (FastAPI). Correcting anisotropy pays where
  anisotropy is the problem.
- Whitening alone is unreliable (helps two corpora, costs saleor 4 points).
- Beyond r≈4 ABTT starts destroying signal.

## Transfer, re-run with a larger pool

saleor (6,496 Python functions) added to the pool. The conclusion hardens:

|  | msegui | fastapi | hono | saleor |
| --- | ---: | ---: | ---: | ---: |
| no projection | 0.7186 | 0.9083 | 0.6512 | 0.6679 |
| trained on msegui | **0.7921** | 0.9037 | 0.6395 | 0.6457 |
| trained on fastapi | 0.7168 | **0.9358** | 0.6512 | 0.6363 |
| trained on hono | 0.7419 | 0.9083 | **0.7209** | 0.6259 |
| trained on saleor | 0.7392 | 0.8991 | 0.6977 | **0.6889** |
| generic (leave-one-out) | 0.7446 (+0.026) | 0.9128 (+0.005) | 0.6744 (+0.023) | 0.6432 (**−0.025**) |

**Per-repo gain +0.048 mean; generic gain +0.007 mean, with a regression.** The
generic matrix is not viable, and this is no longer explainable by a small
training pool.

**ABTT and the projection are redundant**, confirmed on all four corpora: with
ABTT(4) applied first the per-repo gain collapses from +0.074 to +0.022 on
MSEgui and to *zero* on Hono, and the generic case turns negative everywhere but
FastAPI. Ship one or the other, never both.

## Where P2 lands

| configuration | msegui | fastapi | hono | saleor | mean | cost |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| raw static | 0.7186 | 0.9083 | 0.6512 | 0.6679 | 0.7365 | — |
| **+ ABTT(4)** | 0.7590 | 0.8899 | 0.7209 | 0.6654 | **0.7588** | **free** |
| **+ per-repo projection** | 0.7921 | 0.9358 | 0.7209 | 0.6889 | **0.7844** | 1,000 heavy embeds ≈ 43 s CPU / 14 s Metal |

Two shippable points, both far from the 19-minute full heavy index:

- **Pure static, no llama.cpp anywhere**: 0.759 mean. Nothing to download,
  nothing to build, deterministic, model embeddable in the binary.
- **Static + a 43-second alignment sample**: 0.784 mean. Keeps the heavy model,
  but only for 1,000 embeddings at fit and **never at check**.

The same 1,000 pairs also *measure* the residual quality for that specific repo,
so a fit that spends 43 seconds can report the guardrail's real limit instead of
asserting a headline number — which matters, because the static penalty is
repo-specific, not language-specific: **two Python repos measured 0.9083
(fastapi) and 0.6679 (saleor)**.

## `misplaced` under a static index — the expected weak point is the strong one

`redundant` needs only the single best match; `misplaced` polls the top-k
neighbours and votes on which **area** they live in, so it was predicted to
degrade further (static recovers only ~40% of the heavy top-10 set).

The production vote (`placement.rs:221-270`) was replicated offline — area map,
`k` and `z` read from each fitted artifact — and measured two ways:

- **over-fire**: an in-place corpus function should not fire
- **recall**: the calibration's own `sim_recall` test — the same function
  *claimed by an area it does not belong to* should fire

| corpus | k | z | in-place fires (heavy) | decision agreement | heavy recall | static | static + ABTT(4) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| msegui (pascal) | 10 | 1 | 6 / 1500 | 0.9900 / 0.9927 | 0.9727 | 0.9687 | **0.9733** |
| fastapi (python) | 10 | 1 | 0 / 325 | 0.9846 / 0.9938 | 0.9692 | 0.9385 | 0.9415 |
| hono (typescript) | 10 | 1 | 3 / 304 | 0.9901 / 0.9967 | 0.9309 | 0.8388 | 0.8487 |

**The prediction was wrong, and instructively so.** `misplaced` keeps
**90–100%** of the heavy model's recall — on MSEgui the static+ABTT index scores
0.9733 against the heavy model's 0.9727, i.e. no gap at all.

The reason: the vote does not depend on *which* neighbours are returned, only on
*which area* they live in. Swapping a neighbour for a different function in the
same directory leaves the vote unchanged, so overlap@10 — which counts identity
— was simply the wrong metric for this rule. Over-fire is unaffected too
(agreement ≥0.98 on in-place functions everywhere).

So the static penalty is **not uniform across the two semantic rules**:

| rule | what it needs | static retains |
| --- | --- | ---: |
| `redundant` | the single nearest neighbour, exactly | 0.72–0.79 |
| `misplaced` | the neighbourhood's *area*, not its identity | **0.90–1.00** |

## Still open

- A generic matrix trained on tens of thousands of functions across all 12
  languages. The evidence says it will not work, but it has only been tested at
  ~8k training functions across 3 languages.
- End-to-end A/B on the real `redundant`/`misplaced` rules and their fixtures;
  every number here is the matched-gate proxy on the nearest-neighbour task.
- `misplaced` specifically: it votes over the top-10 neighbours, which static
  recovers far less well than the top-1.
