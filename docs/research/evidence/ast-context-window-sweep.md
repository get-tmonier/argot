# AST context window sweep: file_only replaces the 20-line baseline

## Setup

Phase 11 froze the scorer at `EnsembleJepaScorer mean_z` (the
Stage-5 winner), froze UniXCoder, and swept the context window
used to condition JEPA on 2000 FastAPI records. Five modes were
compared against the 51-fixture v2 set (31 break, 20 control, 9
categories) on a single seed: the 20-line lexical `baseline`,
`parent_only`, `file_only`, `siblings_only`, and `combined`. Each
AST mode was rebuilt from the FastAPI git tree with drop rate 0/2000 per mode — no corpus contamination.

## Results

`baseline` (20-line) AUC 0.4871 — below chance. `file_only` AUC 0.6532 (+0.1661) took the lead, and every AST mode beat baseline by ≥ +0.1371. `file_only` uses the full source file minus the hunk with a 2000-char budget.

| mode | AUC | Δ vs baseline |
|:---|---:|---:|
| `baseline` (20-line) | 0.4871 | — |
| `parent_only` | 0.6274 | +0.1403 |
| `siblings_only` | 0.6242 | +0.1371 |
| `combined` | 0.6452 | +0.1581 |
| `file_only` (winner) | 0.6532 | +0.1661 |

Category lifts for `file_only` vs `baseline`: `routing` +0.3334,
`exception_handling` +0.3334, `background_tasks` +0.3750 (from a
0.0000 baseline), `downstream_http` +0.1666. Zero categories regressed. `file_only` fallbacks were mostly truncation events
against the 2000-char budget; `siblings_only` and `combined` had
higher variant-fallback rates for module-level hunks.

## Interpretation

The original 20-line lexical window was actively harmful —
trimming context away from surrounding imports, class definitions,
and sibling endpoints removed exactly the signal JEPA needed. A
file-level window bought the biggest single-step AUC improvement
the signal hunt had produced to date, without any scorer change.
This reframed the Phase 12 question: if context alone moved the
needle this far, how much of JEPA's remaining advantage over cheap
baselines was vocabulary-level to begin with?
