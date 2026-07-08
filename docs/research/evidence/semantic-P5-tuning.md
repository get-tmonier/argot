# P5 — F1/F2 operating-point tuning on production indices — RESULT

Status: **tuned + frozen.** Date: 2026-07-07. Measured against the actual
`.argot/semantic-index.json` argot builds (not the exploration caches), so the
numbers reflect production extraction + calibration exactly. Scripts:
`scratchpad/tune_f1.py`, `tune_f2.py`. Corpora: rich (821 fns), scrapy (1454 fns,
24 areas). Positives: the exploration's blind spec-only reimpls (worst case).

## F1 reinvention — margin-bar percentile sweep

Retrieval (nearest-is-original): **rich 30/30 = 100%, scrapy 27/28 = 96%** — the
load-bearing, generalizing signal (and what powers F4 evidence).

| percentile | rich recall / raw over-fire | scrapy recall / raw over-fire |
|:--|:--|:--|
| 0.90 | 70% / 10% | 61% / 10% |
| 0.95 | 47% / 5%  | 36% / 5%  |
| **0.97 (chosen)** | **30% / 3%** | **18% / 3%** |
| 0.98 (old) | 17% / 1.9% | 18% / 2% |
| 0.99 | 13% / 1% | 14% / 1% |

**Chosen: 0.97.** Raw self-margin over-fire ~3% is a conservative *upper bound* —
real diffs add few, mostly-novel functions, the gates prune boilerplate, and much
of the corpus-self "over-fire" is genuine duplication the feature *should* surface
(A1). Recall is measured on the *hardest* case (blind reimpls); real agent
reinventions embed closer and fire more. 0.97 ~doubles rich recall vs the old 0.98
while holding scrapy, at +1% raw over-fire. (0.95 available for higher recall if
real-world FP proves tolerable — trivially retunable via one constant.)

Known limit (confirmed on rich): a reinvention that lands in an *existing
near-duplicate cluster* (rich has `cell_len`/`_cell_len`/`cached_cell_len`) has a
diluted margin and abstains, even though retrieval nails the original. Margin
firing is conservative by design; retrieval is not.

## F2 placement — threshold sweep (scrapy, 24 areas)

Transplant AUC **0.945** (random foreign area); in-place in-area frac 0.47 vs
transplant 0.03 — a wide, usable gap. But the *fire* thresholds mattered a lot.
First pass used a single-modal-area plurality gate; it wrongly abstained on a
real misplacement whose home **splits across sibling dirs** (an e2e proxy-fn
transplant: neighbours 5× `core/downloader` + 4× `downloadermiddlewares`, so no
single area hit 60%). Switched the concentration measure to the **top-2 areas
combined**:

| ceiling | factor | top2≥ | over-fire (in-place) | recall (transplant) |
|:--|:--|:--|:--|:--|
| 0.20 | 0.40 | – (old) | **15.3%** | 78% |
| 0.05 | 0.30 | 0.6 | 4.5% | 62% |
| 0.05 | 0.30 | 0.7 | 4.1% | 53% |
| **0.05 | 0.30 | 0.8 (chosen)** | **2.5%** | **41%** |

**Chosen: ceiling 0.05, factor 0.30, top-2 concentration ≥ 0.8.** The old
(0.20/0.40, no concentration) fired on 15% of correctly-placed functions —
cross-cutting helpers/mixins/base classes (the R4 risk). Requiring a near-zero
own-area share *and* the neighbours to concentrate in their top two areas cuts
over-fire to **2.5% on scrapy and 0.7% on rich** (flat repo → mostly abstains, as
it should), at ~41% transplant recall. Confirmed e2e: the proxy-fn transplant now
fires — "looks like scrapy/core/downloader code filed under scrapy/commands".

## UPDATE — callee-confirmation breaks the F1 recall/FP frontier

The margin-only rule was honest but weak (rich 27% / scrapy 14% recall on blind
reimpls). Root cause: margin dilutes when the repo already has near-dup clusters,
even though the reimpl's cos₁ is high (0.85–0.97). Absolute-cos alone recovers
recall but over-fires (anisotropy: legit-similar code also sits at cos > 0.88).

**Fix (frontier-breaker):** embedding *retrieves* the candidate, shared **callees**
*confirm* it's a real reinvention. Genuinely-new code shares ~0 callees with its
nearest match; a reinvention shares ~half. New rule: fire if `margin > bar` OR
(`cos₁ ≥ 0.85` AND both sides have ≥2 callees AND `calleeJaccard ≥ 0.15`). Callees
stored per function in the index (`extract_callees`, reused from call-receiver).

Result (real CLI recall + move-gated LOO over-fire, index callees):
| corpus | recall (was) | over-fire |
|:--|:--|:--|
| rich | **70%** (27%) | **1.3%** |
| scrapy | **61%** (14%) | **0.4%** |

Critical gotcha: the **same-name move gate** is essential — without it scrapy
over-fires 7–8% on its repetitive middleware pattern (`process_request`↔
`process_request`); with it, 0.4%. Bars are absolute + stable, so no per-repo
calibration for the callee path. Constants: `CONFIRM_SIMILARITY=0.85`,
`CALLEE_OVERLAP_BAR=0.15`, `MIN_CALLEES_FOR_CONFIRM=2` (margin path + bar kept as OR).

## UPDATE 2 — IDF-weighted subtokens + two-tier rescue → ~86% recall

The callee-only confirm (above) plateaued at rich 56% / scrapy 52% on the harder
36/29-reimpl sets. Two changes broke through to **86% on both** at ~2% over-fire
(sweep: `scratchpad/sweep_recall.py`, real fitted callees + subtokens):

1. **IDF-weighted subtoken overlap** as a second confirm signal. Identifiers are
   split into subtokens (`getUserName`→`user`,`name`; camelCase + snake_case +
   acronyms), weighted by corpus rarity (IDF). A shared *rare* domain token
   (`east_asian_width`) is strong reinvention evidence; shared ubiquitous ones
   (`self`/`get`/`return`) carry ~0 weight — so **no per-language stop-list** is
   needed (language-agnostic, verified: dropping the keyword filter *improved*
   FP). Plain (unweighted) token overlap hit the same recall but at 3.0% over-fire
   (rich); IDF-weighting cut that to 0.7% — a 4× FP reduction at equal recall.

2. **Two-tier rule.** Normal tier (`cos₁≥0.78` + moderate structure) OR a strong
   *rescue* tier (`cos₁≥0.70` + high structure) that catches heavily-reworded
   reinventions embedding further from the original but still sharing rare
   vocabulary. Retrieval ceiling (nearest==original file) is rich 81% / scrapy 90%
   on blind reimpls — the strong tier + firing on valid near-dups lets recall meet
   or approach that ceiling.

Frozen constants (redundant.rs): normal `SIM=0.78 / SUBTOKEN=0.40 / CALLEE=0.12
(≥2 each)`; strong `SIM=0.70 / SUBTOKEN=0.52 / CALLEE=0.30 (≥3 each)`. The **margin
path was removed** (its 0.97-percentile bar fired on ~3% of the corpus by
construction — pure additive FP now that subtokens carry callee-less standouts).
`margin_bar` calibration + artifact field deleted. Subtokens stored per function
in the index alongside callees; IDF computed at scorer construction.

### Validated on the real shipped binary (not the sweep model)

Re-fit rich/scrapy/fastapi/ink with the release+semantic binary (subtokens now
stored in the index), then measured recall via the real CLI (`sem_bench.py`,
end-to-end `argot check`) and over-fire via `sem_analysis.py` (production gates:
dunder + test-path + move-gate):

| corpus | recall (real CLI) | over-fire (gated LOO) | note |
|:--|:--|:--|:--|
| rich   | **90.0%** (27/30) | **1.8%** | tuned |
| scrapy | **89.3%** (25/28) | **1.6%** | tuned; a fire landed at cos 0.72 — strong-tier rescue working |
| ink (TS) | — | **0.4%** | **held-out — generalises on TypeScript** |
| fastapi | — | 4.5% | **held-out — elevated, but verified genuine duplication** |

Both tuned corpora **beat the 85% target** on the real binary (higher than the
sweep's 86% because the CLI plants reimpls that embed slightly closer than the
LOO-model estimate). ink (a held-out TS corpus) confirms the language-agnostic
subtoken split generalises.

**fastapi is duplication-dense, not a false-alarm problem.** Its 4.5% self-fire is
uniform across `fastapi/` (4.2%), `docs_src/` (4.3%) and `scripts/` (5.6%) —
verified by inspection: `fastapi/params.py` holds 8 near-identical param-class
`__init__`s (Path/Query/Header/Cookie/Body/Form/File), and each pairs with a
near-identical factory in `param_functions.py` (`class Body` ↔ `def Body()`), plus
`generate_operation_id` ↔ `generate_operation_id_for_path` (sub-Jaccard 0.83).
These ARE reinventions the feature should surface; the LOO self-fire metric counts
genuine internal duplication, exactly as the RUBRIC notes.

**The callee path is load-bearing, not redundant.** A subtoken-only rule collapses
rich recall to 47% (blind reimpls heavily rename identifiers but reuse the same
helpers → callee overlap catches them; subtoken overlap alone misses them). Every
FP-tightening variant that trims fastapi also craters rich recall — rich's real
reinventions and fastapi's param family sit in the *same* callee-overlap band
(cj 0.12–0.25), so no threshold separates them. The current rule is the recall-max
frontier; trading 22 pts of rich recall to shave ~1% off one duplication-dense
corpus's (largely genuine) self-fire is the wrong trade.

## Verdict

Both channels tuned to a **low-over-fire advisory operating point** (~1–3% raw,
much lower in production), consistent with argot's low-false-alarm identity while
staying their own advisory channels (not the base foreign-catch budget). Retrieval
(F1) and placement separation (F2) both generalize across two independent corpora.
Frozen constants: `MARGIN_BAR_PERCENTILE=0.97`, placement
`ABS_IN_AREA_CEILING=0.05 / MISPLACED_FACTOR=0.30 / MIN_TOP2_CONCENTRATION=0.80`.
