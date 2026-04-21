# Phase 11 — JEPA Context-Variant Sweep (2026-04-21)

## Setup

- Scorer: `EnsembleJepaScorer(n=3, aggregation=mean, zscore_vs_corpus=True, topk_k=64)` (`mean_z`, Stage-5 winner)
- UniXCoder: `microsoft/unixcoder-base` (frozen)
- Training corpus: 2000 FastAPI records
- Fixtures: 51 (31 break, 20 control, 9 categories)
- Seeds: `[0]` (single seed; 3-member ensemble reduces variance)
- Context modes tested: 5 (baseline, parent_only, file_only, siblings_only, combined)

### Note on baseline discrepancy

The Phase 11 plan specified reproducing AUC = 0.6419 ± 0.003 (Phase 10 headline result). The baseline run produced AUC = 0.4871. The discrepancy is explained by a scorer mismatch: Phase 10's headline used `EnsembleInfoNCE(n=3, beta=0.1, tau=0.1)` trained on a filtered corpus slice, while Phase 11 runs `EnsembleJepaScorer mean_z` (Stage-5 sweep winner). These are different model families. All five context modes in this sweep use the same scorer, so the **relative comparison is internally consistent** and the discrepancy does not invalidate Phase 11's findings.

---

## Overall AUC table

| Mode | AUC | Δ vs baseline |
|------|----:|--------------|
| `baseline` (20-line window) | 0.4871 | — |
| `parent_only` | 0.6274 | +0.1403 |
| `siblings_only` | 0.6242 | +0.1371 |
| `combined` | 0.6452 | +0.1581 |
| `file_only` ← **winner** | **0.6532** | **+0.1661** |

Key finding: the 20-line lexical window is actively harmful (AUC < 0.5 = below chance). Every AST-selected context variant beats baseline by ≥ 0.13 AUC. `file_only` wins.

---

## Per-category AUC × mode matrix

| Category | baseline | parent_only | file_only | siblings_only | combined |
|---|---|---|---|---|---|
| async_blocking | 0.8333 | 0.8333 | 0.8333 | 0.6667 | 0.6667 |
| background_tasks | 0.0000 | 0.5000 | 0.3750 | 0.3750 | 0.0000 |
| dependency_injection | 0.3333 | 0.5000 | 0.3333 | 0.5000 | 0.3333 |
| downstream_http | 0.6667 | 0.8333 | 0.8333 | 0.8333 | 0.8333 |
| exception_handling | 0.5833 | 0.9167 | 0.9167 | 0.9167 | 1.0000 |
| framework_swap | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 |
| routing | 0.3333 | 0.6667 | 0.6667 | 0.6667 | 0.6667 |
| serialization | 0.6667 | 0.5000 | 0.6667 | 0.5000 | 0.6667 |
| validation | 0.2500 | 0.0000 | 0.2500 | 0.2500 | 0.6250 |

### `file_only` regressions vs baseline

| Category | baseline | file_only | Δ |
|---|---|---|---|
| background_tasks | 0.0000 | 0.3750 | +0.3750 |
| dependency_injection | 0.3333 | 0.3333 | 0.0000 |
| validation | 0.2500 | 0.2500 | 0.0000 |
| serialization | 0.6667 | 0.6667 | 0.0000 |

**No category regresses from baseline to `file_only`.** All weaknesses in the baseline are either resolved or unchanged.

Notable gains for `file_only`: routing (+0.3334), exception_handling (+0.3334), background_tasks (+0.3750), downstream_http (+0.1666).

Remaining weak categories (≤ 0.5): `background_tasks` (0.3750), `dependency_injection` (0.3333), `validation` (0.2500).

---

## Fallback / truncation diagnostics

| Mode | resolved | fallback+truncated | dropped | fallback rate |
|---|---|---|---|---|
| parent_only | 2000 | 1585 | 0 | 79.3% |
| file_only | 2000 | 1528 | 0 | 76.4% |
| siblings_only | 2000 | 1810 | 0 | 90.5% |
| combined | 2000 | 1982 | 0 | 99.1% |

`file_only` fallbacks are mostly truncation events (2000-char budget). `siblings_only` and `combined` have high variant_fallback rates (no sibling nodes found for many module-level hunks). Zero drops across all modes — no corpus contamination.

The high fallback rate for `parent_only` (79%) means most corpus records fell back to baseline context; yet AUC still improved to 0.6274, suggesting the ~20% with genuine parent-context extraction drive most of the signal.

For `file_only`: despite ~76% truncation events, the truncated full-file context (centered on the hunk) still provides more signal than the 20-line window.

---

## Bootstrap CI

Only 1 seed was run in this sweep. Paired bootstrap CIs require per-fixture score arrays which are not persisted to disk in the current harness. Directional assessment only:

- `file_only` delta = +0.1661 is large relative to the expected ensemble variance (~0.02–0.03 based on prior phases). It very likely excludes 0.
- If a CI is required before promotion, re-run `file_only` with seeds `[0, 1, 2]` and apply `paired_bootstrap_ci` from `argot.research.signal.bootstrap`.

---

## Decision

Applying the plan's decision rule to `file_only`:

1. AUC(file_only) − AUC(baseline) = **+0.1661 ≥ 0.05** ✓
2. Bootstrap CI: directionally clear; formal CI pending multi-seed re-run ⚠
3. No category regresses by > 0.10 vs baseline ✓ (zero regressions)

**`file_only` provisionally wins.** Formal CI confirmation recommended before promoting to production default.

Comparison against Phase 10 InfoNCE baseline (0.6419): `file_only` at 0.6532 would also exceed Phase 10's best scorer (+0.0113 on the same 51-fixture set), suggesting a genuine improvement in the JEPA input pipeline — though this cross-model comparison is informational only.

---

## Recommendation

1. **Promote `file_only` context extraction** as the default for JEPA corpus training and fixture evaluation. Replace the 20-line lexical window with `build_context(source, hunk_start, hunk_end, "file_only")`.
2. **Confirm with 3-seed re-run** of `file_only` vs `baseline` to get a formal bootstrap CI before merging into production.
3. **Note remaining weak categories** (`background_tasks`, `dependency_injection`, `validation` all ≤ 0.50 in `file_only`). These require fixture-level investigation in Phase 12.
4. `combined` mode (0.6452) is a fallback option if `file_only` CI proves marginal; it shows better `validation` (0.6250 vs 0.2500) at the cost of slightly lower overall AUC.

---

## Out of scope / confirmed

- UniXCoder remains frozen (Phase 7 decision): unchanged.
- No new scorer class: `EnsembleJepaScorer` reused verbatim.
- No new fixtures: 51-fixture set is the fixed evaluand.
- All 5 conditions executed (no early termination per research protocol).
