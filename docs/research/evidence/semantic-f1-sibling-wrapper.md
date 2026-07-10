# Semantic layer — F1 reinvention: sibling / wrapper false-alarm filters

**Date:** 2026-07-09 · **Branch:** `feat/semantic-layer` · commit `bcf6b61b`

## Why this exists

The first clean-commit audit (`semantic-f1-clean-commit-fp.md`) left F1 firing on
5–14 %/hunk of real commits on library/framework corpora — above the ≤5 % bar on 11
of 31 corpora. This pass drives the false-fire down with three cheap structural
filters found by capturing per-fire features on the actual fires and separating the
false population from the planted recall population, with **recall fully preserved**.

## What the false fires are (per-fire feature capture)

A dev-only `ARGOT_DBG_REINV` trace dumped every fire's full feature set (cosine,
callee/subtoken Jaccard, callee counts, **matched-symbol repo frequency**, **embedding
neighbour density**, body size). Captured on the failing corpora (FP = clean-commit
fires) and their planted reimplementations (recall). Three non-reinvention shapes
dominate the false population:

1. **Thin wrappers / accessors** — `fn lower(&self){ self.floor() }`. On guava,
   157 of 178 fires were <8-line delegators (`floor↔lower`, `higher↔ceiling`).
2. **Interface / family methods** — `on_send` (defined **271×** across rubocop's
   cops), `autocorrect` (138×), assertion overloads (junit5 `assertSame↔assertEquals`,
   defined **123×**), `ReadMetadata` across jellyfin's `*Provider` classes.
3. **Dense sibling clusters** — redis `*Command` handlers, curl connection-filter
   `create/destroy/recv`, saleor per-entity `resolve_*`. Unique names, but the function
   sits in a **crowd of near-identical siblings** (recall fires have ≤2 near neighbours;
   sibling FPs up to 10).

## The filters (pure Rust, zero inference cost — `redundant.rs`)

Each rejects one shape, and each is **exempted when the candidate reuses the match's
exact helpers** (callee overlap), which is what a genuine reimplementation of one
specific member does:

- `MIN_REINVENTION_BODY_LINES = 6` — a <6-line body is a wrapper (under the
  weak-overlap guard: callee < 0.30 and subtoken < 0.40).
- `FAMILY_SYMBOL_DF = 5` (weak-overlap guard) / `VERY_FAMILIAR_SYMBOL_DF = 20`
  (unconditional, callee < 0.50 exempt) — a method defined many times is an interface
  method, not something you reinvent.
- `DENSE_CLUSTER_NEIGHBORS = 3` (weak-overlap guard) / `VERY_DENSE_NEIGHBORS = 7`
  (unconditional, callee < 0.50 exempt) — a function in a crowd of near-identical
  neighbours is a family member.

The two guards were tuned to keep genuine reimplementations: a `password` generator
in a dense family (`generatePassword↔password`, callee 0.44) survives the moderate
tier; a `cyclicShift↔rotate` (callee 1.0) survives the very-dense tier via the
exact-helper exemption; a 5-line `group_types_by_encoder` (callee 1.0) survives the
body filter.

## Results

**Recall fully preserved** — 0 of the planted reimplementations dropped across 13
corpora incl. the family-heavy rubocop/guava/junit5 (verified on captured pre-gate
recall fires + smoke on the real binary: redis 94 %, faker-js 85 % unchanged).

**Clean-commit false-fire, window 150 (raw; 3-judge labelled true-FP for the residual
corpora, majority vote, default false-alarm):**

| corpus | true-FP before → after | | corpus | true-FP before → after |
|---|---|---|---|---|
| rubocop | 12.6 → **4.9 %** | | rocksdb | 7.3 → **5.0 %** |
| saleor | 6.7 → **1.2 %** | | ripgrep | 7.4 → **4.4 %** |
| redis | 8.0 → **4.6 %** | | guava | 5.9 → **4.6 %** |
| junit5 | 9.5 → **5.0 %** | | homebrew | 6.0 → **4.0 %** |

(rubocop and junit5 3-judge labelled on the survivors; the rest are raw ≤5 %, an
upper bound on true-FP.) **28 of 31 corpora land ≤5 % true-FP** — every corpus except
three. Those three — **curl 6.2 %, jellyfin 7.0 %, laravel 6.6 %** (3-judge labelled) — Their residual fires are, by human
review, **parallel backends** (curl openssl↔wolfssl session cache), **sibling-module
methods** (laravel Illuminate/*), and **per-provider / sync-async twins** (jellyfin) —
code that even a skeptical reviewer classifies as "not a reinvention, but structurally
identical." This is the documented irreducible floor of a retrieval-plus-structure
sense: ~⅓ of the false alarms cannot be separated from genuine reinvention without
semantic understanding, which a name/structure guardrail (no LLM, <100 MB binary) does
not have. Advisory; base guardrail untouched.

## Post-hoc: two more levers tried on the 3 residuals — both rejected (2026-07-09)

Before closing the residuals as irreducible, two additional language-agnostic signals
were tested against the captured per-fire features (`feats/*_fp.tsv` vs
`*_recall.tsv`, no rebuild):

1. **Unconditional very-high-df tier** (kill any match to a symbol defined ≥30–50×,
   *without* the callee-overlap exemption). Exhausted: curl and redis residuals carry
   **zero** high-df fires (unique-named parallel backends); junit5/jellyfin's high-df
   fires (`assertNotEquals` df 123, `GetMetadata` df 20) are **already** killed by the
   existing `VERY_FAMILIAR_SYMBOL_DF` filter. No residual left to cut.

2. **Directory-relationship filter.** The residual false fires are overwhelmingly
   **same-directory** (curl 22/28) or same/sibling/nested (jellyfin 37/57), while every
   planted recall fixture reads as *far-dir* — so a "suppress same-dir matches" rule
   scores **0 measured recall loss** and would push all three corpora ≤5 %. **Rejected
   as eval-overfitting:** the recall harness plants every fixture in one `_sembench/`
   dir at the repo root (`sem_bench.py`), so it *structurally cannot* place a
   reimplementation in the same directory as its target — the "0 recall loss" is a
   blind spot, not a result. An *unconditional* same-dir suppression would also blind
   the sense to the most catchable real reinvention (an agent copying a helper into a
   neighbouring file, high overlap: curl's `ssh_pollset` cos 1.00/callee 1.00 lives in
   sibling ssh backends). The defensible narrow form (same-dir **and** weak overlap)
   catches only ~half the residuals (curl 10/22) — not enough to clear the bar. Cutting
   the rest would mean gaming a metric the harness can't adjudicate, which the project
   forbids.

**Verdict:** the 3 residuals are same-directory structural twins of a *local family*
pattern (parallel backends, provider twins). By argot's own north star — "code foreign
to the repo's own patterns" — a same-directory near-identical sibling is the
*least*-foreign shape there is, so flagging it is closer to a definitional edge than a
tunable bug. No non-LLM feature separates it from genuine same-directory reinvention
without an eval blind spot. Floor confirmed; prod code unchanged.
