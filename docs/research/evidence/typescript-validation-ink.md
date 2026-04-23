# TypeScript validation: ink (terminal React renderer)

## Setup

Second TS validation after hono. ink is vadimdemedes/ink, a TSX-heavy
terminal rendering library built on React reconciler — a very different
shape from hono (HTTP framework with middleware modules). Same
`SequentialImportBpeScorer` + `LanguageAdapter` scorer that produced
the hono result; same protocol of scoring 5 recent PRs end-to-end
against each PR's pre-merge snapshot. The sampler was already extended
from the hono run to include `variable_declarator` nodes with arrow
or function-expression RHS, which matters here too — ink exports most
components as `export const Foo = () => {...}`.

## Results

**3 source flags across 14 source hunks from 5 PRs = 21.4% flag rate**
(below the 30% investigation threshold). All 3 flags are on newly
introduced feature content, not pre-existing patterns.

| PR | Title | src hunks | flagged | BPE threshold | pool |
|---|---|---:|---:|---:|---:|
| #937 | fix: Respect disableFocus() on Escape | 1 | 0 | 3.7528 | 141 |
| #906 | feat: add border background color support | 8 | 2 | 3.7459 | 140 |
| #925 | feat: add wrap="hard" to Text component | 3 | 1 | 3.7517 | 141 |
| #910 | fix: incremental rendering trailing newline | 1 | 0 | 3.7099 | 135 |
| #879 | fix: mark text node dirty on insertBefore | 1 | 0 | 3.5422 | 113 |

Stage 1 import fired 0 times on source — the TypeScript adapter
resolved ink's own package as `{'hono'...}` analogue correctly; all
internal imports dispatched as repo-internal. BPE threshold band
3.5422–3.7528 (Δ = 0.211) — wider than hono's Δ = 0.014, driven by
snapshot variation across a smaller source tree (56–64 non-test
`.ts`/`.tsx` files). Calibration pool was pool-capped at 113–141; the
mandatory 3-seed stability probe returned identical thresholds (σ =
0.000) on every PR. Data-dominance excluded 2/64 source files (3.1%):
one legitimate linter config (`xo.config.ts`), one documentation false
positive on `src/output.ts` whose JSDoc string matched an
auto-generated marker.

## Interpretation

TSX-heavy React library shape didn't destabilise the scorer. Flag rate
well under the investigation threshold, feature-introduced flags only,
deterministic calibration — the `LanguageAdapter` seam generalised
cleanly from hono's middleware shape to ink's component shape.
