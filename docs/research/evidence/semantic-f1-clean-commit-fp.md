# Semantic layer — F1 reinvention clean-commit false-positive audit + fix

**Date:** 2026-07-08 · **Branch:** `feat/semantic-layer`

## Why this exists

Prior notes recorded the F1 reinvention channel's clean-commit false-positive rate
as "LOW and MOSTLY GENUINE" — the belief that when `redundant` fires on a real
developer commit, it is mostly surfacing genuine internal duplication, not a false
alarm. That belief rested on one TS corpus (excalidraw) plus spot-checks. This
audit measured it **honestly across all 31 corpora** with a leak-free temporal
holdout and **adversarial labelling of every fire**, and the belief did not hold:
on framework/business-logic-heavy corpora F1 over-fires substantially, and most of
those fires are **not** genuine duplication.

## Method

1. **Clean-commit FP** (`benchmarks/sem_fp.py`): fit the semantic index at
   `HEAD~150` (first-parent), replay every non-merge commit strictly after the fit
   point through the real `argot check --commit`, count `redundant` fires. A real
   dev commit is new work, so a fire is a candidate false alarm. Window 150 fixed
   for all corpora; restores the clone after. Each fire is emitted as a structured
   record (reinventing fn path:line, matched fn symbol/path:line, similarity).
2. **Adversarial labelling**: for every fire, three **independent** skeptical
   sub-agent judges read the reinventing fn (from its commit) against the matched
   existing fn (from the fit tree) and vote genuine-reinvention vs false-alarm,
   **defaulting to false-alarm when unsure**. Majority (≥2/3) decides. This yields
   a conservative TRUE-FP upper bound and a genuine-catch lower bound.
3. **Signal attribution** (`ARGOT_DBG_REINV` trace in `redundant.rs`): for each
   fire, dump the confirming tier and the shared callees/subtokens with their corpus
   df/idf, so the genuine set can be separated from the false-alarm set structurally.

## Baseline finding — F1 over-fires broadly, across every language

Clean-commit FP at window 150 across all 31 corpora, raw `redundant` fires per
scanned hunk. **22 of 31 corpora exceed 2%/hunk**, spanning every language — this
is *not* a Python- or duplication-heavy-only effect:

| tier | corpora (raw redundant/hunk) |
|---|---|
| **>8%** | rubocop 14.9 · jellyfin 11.8 · saleor 11.0 · junit5 10.0 · curl 9.7 · laravel 9.4 · redis 8.7 · ripgrep 8.6 |
| **4–8%** | guava 5.9 · gh-cli 5.7 · wagtail 5.5 · dagster 5.1 · scrapy 4.7 · eslint 4.3 |
| **2–4%** | excalidraw 4.0 · homebrew ~ · hugo 3.1 · fmt ~ · ink 2.6 · outline 2.1 · composer ~ · rocksdb ~ |
| **<2%** | powershell 1.8 · fastapi 1.5 · bat 1.3 · commander 1.2 · hono 0.8 · rich 0.7 · faker 0.6 · faker-js 0.4 · express 0 |

**Labelled TRUE-FP (3-judge majority, default false-alarm):**

- **scrapy**: 68 fires → **3 genuine, 65 false-alarm** (4.45% true-FP/hunk). The 3
  genuine: `md5sum`↔`_md5sum` (sim 0.95, verbatim), `_iter_command_classes`↔
  `iter_spider_classes` (with an in-code TODO to merge), an SSL-logging block copy.
- **saleor** (34-fire representative sample): **9 genuine, 25 false-alarm** (~26%
  genuine → ~122 false of 166, ~8%/hunk). Genuine were exact/near-exact copies
  (similarity 1.0) and standalone reimplementations of existing webhook/tax
  orchestration.

So F1's clean-commit **precision is low on these corpora** — ~4% (scrapy) to ~26%
(saleor) of fires are genuine. The rest are false alarms.

## Root cause — confirmation fooled by shared *common* structure

Signal attribution cleanly separates the two populations (means):

| signal | GENUINE | FALSE-ALARM |
|---|---|---|
| callee Jaccard | **0.71** | 0.20 |
| # shared callees | **3.5–5.3** | 1.5–1.9 |
| min shared-callee df (rarity) | **1.7** (rare helpers) | 16.6 (common) |
| subtoken Jaccard | 0.47–0.82 | 0.24–0.37 |
| firing tier | normal / strong | **normal + rare-callee** |

The false alarms are short functions that share a **framework idiom**, not logic:

- **deprecation shims** — `warn(...); return deferred_from_coro(self.X_async())` —
  every one shares the same two-call skeleton (`warn`, `deferred_from_coro`) while
  wrapping a different operation;
- **sync/async & Twisted/asyncio twins** that must coexist;
- **sibling interface methods** (a `close_spider` per class, file-vs-image pipeline
  specializations);
- **per-codec decompressors** (`_inflate`/`_unzstd`/`_unbrotli` vs `gunzip`).

Two firing paths admit them:

1. **The rare-callee path** (28/65 scrapy FPs) fired on a *single* borderline-rare
   callee. At `RARE_CALLEE_DF_FRACTION = 0.012` (≤1.2% of the repo) framework
   utilities like `deferred_from_coro` (df ~1%) still counted as "rare," so one
   shared framework helper at cos ≥ 0.70 confirmed. Genuine catches share helpers at
   df ~1–2 — an order of magnitude rarer.
2. **The callee-Jaccard path** with a single shared *common* callee. The guard only
   required each side to have ≥2 callees, so one shared helper (`warn`, df huge)
   could clear `callee_jac ≥ 0.12`. Genuine catches share **several** helpers
   (Jaccard 0.71, 4–8 shared).

## The fix (principled, and bounded by the recall frontier)

Both leaks are the same mistake: **treating a shared _common_ helper as reinvention
evidence.** Two candidate changes were considered; only one survived the recall test.

**Shipped — `RARE_CALLEE_DF_FRACTION 0.012 → 0.004`.** The single-shared-callee path
now requires a *genuinely* rare helper (≤0.4 % of the repo, an order of magnitude
below the old 1.2 %), excluding borderline framework utilities (`deferred_from_coro`,
df ~1 %) that drove ~40 % of scrapy's clean-commit false fires while keeping the
distinctive domain helpers a real reimplementation reuses. Recall-neutral: the
genuine single-rare-helper reimpls this path carries share helpers at df ~1–2, far
below the new bar (validated — no corpus regressed below 85 %).

**Tried and reverted — `MIN_SHARED_CALLEES = 2`** (the callee-Jaccard path needs ≥2
*shared* callees). This is the natural reading of the data — genuine catches share
4–8 callees, false alarms ~1.5 — and it roughly halved scrapy's FP. But it hit the
**recall frontier from the other side**: faker's data-generator library reuses a
*single common* helper (`arrayElement`, `datatype.number`) per generator, which is
structurally *identical* to the dominant false alarm. The rule can't separate them,
so it dropped **faker-js recall 90 → 75 %** for ~13 fewer scrapy FPs (0.9 % of hunks)
— a bad trade that violates the ≥85 % floor. Reverted; the FP it targeted (a single
shared *common* callee) is left to the advisory framing, since suppressing it
necessarily suppresses genuine same-shape reinventions.

The subtoken path and the tier cosines are unchanged (the subtoken path carries
renamed reimpls and is not the leak — FP subtoken Jaccard 0.24–0.37 sits below the
0.40 bar). Mirrored in `benchmarks/sem_analysis.py`.

## The irreducible floor (honest limit)

Simulating candidate rules over the 12 genuine + 90 labelled false alarms
(`scratchpad/sim_rules.py`): the most aggressive structural rule that keeps **all**
genuine catches still leaves **~33/90 false alarms**. About **37% of the false
alarms are structurally indistinguishable from genuine reinventions** — sibling
interface methods and parallel business modules genuinely *do* share helpers and
vocabulary; only their intent differs, which embeddings + structural signals cannot
see. So a principled tightening roughly **halves** F1 clean-commit FP but cannot
eliminate it. This is the honest limit of a retrieval-plus-structure reinvention
sense, and the reason the F1/F2 findings are surfaced as **advisory** rather than
gated (see below).

## Results after the fix (validated)

_Pending final validation sweep — recall (held ≥…) and clean-commit FP (before →
after) per corpus._
