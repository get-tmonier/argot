# Aligning train's corpus to `argot:recommended` regresses over-fire — reverted

**Date:** 2026-07-06
**Verdict:** ❌ Do Not Retry — keep the train↔check scope divergence.

## Hypothesis

The fresh-eyes setup dogfood (argot + effract) found that `train` collects a
*different* file set than `check`/`calibrate`/`inspect` judge: train applies its
own narrow `EXCLUDE_DIRS` + test-file patterns, but **not** the built-in
`argot:recommended` set. So `examples/`, `docs/`, and `*.config.*` land in the
*trained* voice even though check excludes them. On effract (a monorepo) the
trained voice was 58% peripheral code (landing site, demos, config), and a
blatant foreign import (`axios`) slipped past.

Hypothesis: make `train` honor the same resolved suppression set (recommended +
`.argotignore`) as check — "lock-step" — so the model learns from exactly the
files it judges.

## Method

Scoped bench (`argot-bench --corpus fastapi,rich,hono,ripgrep`): production-path
novel-pattern catch **and** temporal-holdout FP, compared head-to-head against
the `main` baseline (no change).

## Result

| Metric | Baseline (main) | With train↔recommended | Δ |
|---|---|---|---|
| Novel-pattern catch (headline) | 66/83 (79.5%) | 66/83 (79.5%) | **0** |
| Legacy catch (secondary) — fastapi | 17/32 (53%) | 23/32 (72%) | +19pp |
| Legacy catch — rich | 11/16 (69%) | 12/16 (75%) | +6pp |
| **Over-fire (FP existing-file) — fastapi** | **1.28%** (22/1718) | **2.56%** (44/1718) | **+1.28pp ✗** |
| Over-fire (FP overall) — rich | 0.88% | 1.32% | +0.44pp ✗ |
| Over-fire — hono / ripgrep | 0.00% / 0.63% | 0.00% / 0.63% | 0 |

Catch is unchanged; the change is a **false-alarm regression**. fastapi's
existing-file over-fire *doubles* to 2.56%, breaking the ≤ 0.98% worst /
≤ 2% per-corpus over-fire commitment.

## Why

On a single-package repo, `docs/` and `examples/` are **authored code written in
the repo's own voice** — genuine training signal. Excluding them from training
removes signal, so more of the repo's own existing code reads as foreign at
check time → higher over-fire. The recommended set is right for **check scope**
(don't flag your own docs/examples) but wrong for **training scope** (learn from
all authored code).

The effract monorepo case is different in kind: its `landing/`, `playground/`,
`apps/` are *peripheral packages* in a *different* voice, not the library's. That
is a per-repo judgment, handled by the LLM writing `.argotignore` at setup —
which `train` already honors (user patterns). It must not be generalized into a
blanket rule that also drops good signal on normal repos.

## Decision

- **Keep the divergence.** `train` learns from all authored code minus its
  build/dep excludes and the user's `.argotignore`; `check` additionally hides
  the recommended set from *output*. This is beneficial, not a bug.
- Monorepo pollution is solved by the **setup prompt / `argot-setup` skill**,
  fine-tuned in the same change to make the LLM identify the primary package and
  exclude peripheral workspace members via `.argotignore`.
- **Do Not Retry:** do not apply `argot:recommended` to corpus collection.
