# TypeScript bring-up: hono validation

## Setup

Phase 14 closed by generalising the sequential pipeline across languages
via a `LanguageAdapter` refactor — same two-stage scorer core, swappable
per-language seams for hunk extraction, import resolution, and data /
auto-generated file filtering. The first cross-language validation ran
on honojs/hono, a TypeScript HTTP framework. Five PRs were scored end
to end against each PR's pre-merge snapshot: #4883 (aws-lambda header
fix), #4848 (strong-to-weak ETag conversion), #4750 (bearer-auth regex
escaping), #4780 (jsx-renderer function-based options), and #4834
(css classNameSlug option). N_CAL was 485 hunks (pool size 488–494),
sampler extended to include `variable_declarator` nodes with arrow /
function-expression RHS because hono exports most logic as
`export const foo = () => {...}`.

## Results

0 source flags across 22 source hunks from 5 PRs.

| PR | src hunks | src flags | BPE threshold |
|:---|:---:|:---:|---:|
| #4883 aws-lambda | 1 | 0 | 5.8628 |
| #4848 compress | 1 | 0 | 5.8514 |
| #4750 bearer-auth | 3 | 0 | 5.8491 |
| #4780 jsx-renderer | 2 | 0 | 5.8487 |
| #4834 css | 15 | 0 | 5.8538 |

Stage 1 import fired zero times — `TypeScriptAdapter.resolve_repo_modules`
correctly returned `{'hono'}` as the repo's own exact-match package,
and all internal imports (relative `./`/`../`, plus `hono`, `hono/compress`,
etc.) dispatched as repo-internal. Threshold range 5.8487–5.8628
(Δ = 0.014) — tighter than either Python corpus. Three test-file flags
were all judged INTENTIONAL_STYLE_INTRO (new test suites for the new
feature introduced in the same PR); 0 source flags, 0 AMBIGUOUS, 0
FALSE_POSITIVE.

## Interpretation

The `LanguageAdapter` refactor carried the scorer to TypeScript without
changing the two-stage core. Clean 0% source-flag rate on in-domain
fix/feat PRs, stable thresholds, and no import-axis dispatch errors —
the seam holds.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/phase14/experiments/ts_validation_hono_2026-04-22.md`.
Re-written here for clarity, not copied.*
