# Foreign-concurrency reclassification: raw-builtin misuse is secondary, not gated

**Date:** 2026-07-09 · **Branch:** `feat/semantic-layer` · scope: `benchmarks/foreign_consolidate.py` (`norm_class`), `landing/src/data/foreign.json`, `benchmarks.astro`

## Finding

While expanding the thin foreign-concurrency fixture cells (see the fixture-floor
commit), an audit of `norm_class` in `foreign_consolidate.py` showed it folded **40
raw-builtin-misuse fixtures across 17 corpora** into the **gated** `foreign_concurrency`
catch metric. Categories affected: `wrong_concurrency` (raw `pthread`/`std::thread`
where the repo has its own wrapper), `async_blocking` (a blocking call inside async
code), `sleep_polling` / `thread_background` (busy-wait), `threading` /
`threading_provider` / `threading_concurrency` (the stdlib), `scheduling`.

The fold was a substring quirk: `norm_class` matched `"concurren"/"async"/"thread"/
"sleep"/"schedul"` → `foreign_concurrency` for any category that wasn't *literally*
`foreign_concurrency` (the genuine foreign-library fixtures, which the GATED loop above
already routes correctly).

## Why this is wrong (per RUBRIC.md, pre-registered before scoring)

`RUBRIC.md` defines the gate precisely:
- **`foreign_concurrency` (gated):** "a **foreign concurrency library/runtime** the repo
  does not use … an unattested foreign callee, **not a raw language builtin**."
- **`semantic_convention` (secondary, never gated):** "misuse of the repo's own/attested
  vocabulary: a builtin the repo avoids (`die`/`exit`) … a **proven local limit**;
  reported, never gated."

Raw `pthread`-instead-of-`port::Thread`, a blocking call in async code, or the stdlib
`threading` module are *builtin misuse the repo avoids* — unambiguously the SECONDARY
class, exactly parallel to `wrong_api_within_known` (already routed to
`semantic_convention` in the same function). Folding them into the gate contradicted
the pre-registered spec.

## Fix

One `norm_class` change: a concurrency-flavoured category that is **not** literally
`foreign_concurrency` now routes to `semantic_convention` (secondary), with a comment
citing the RUBRIC. Genuine `foreign_concurrency` (a named foreign lib) is untouched.

## Impact (visible tier — the published cell)

| | catch |
|---|---|
| **Before** — foreign libs + builtin-misuse blended (gated) | 173/189 = **91.5%** |
| **After** — foreign concurrency **libraries** only (gated) | 157/157 = **100%** |
| Builtin-misuse concurrency → moved to secondary (ungated) | 16/32 ≈ **50%** |

Per-corpus, every gated concurrency cell is now 100% (visible) — argot catches foreign
concurrency *libraries* perfectly; the ~50% builtin-misuse was always the drag.

## Not a stat trick — transparency safeguards

- The change makes code match a **pre-registered** spec (fixed before any fixture was
  scored); it is a bug fix, not a rubric amendment.
- It is **directionally the honest read**, not a cosmetic lift: the old blended 91.5%
  *hid* two distinct truths (100% on foreign libs, 50% on builtin-misuse). Splitting
  reveals both.
- **Nothing is deleted.** The 40 builtin-misuse fixtures stay measured and reported in
  the ungated `semantic_convention` class; the landing's "what argot deliberately does
  not gate on" section now names raw-builtin concurrency antipatterns explicitly and
  states the ~50% catch as a documented local limit.
- Consistent with the identical treatment of `wrong_api_within_known` and
  `wrong_error_discipline` (both already secondary).

Base guardrail untouched; no scorer/prod-code change (consolidation + landing only).
