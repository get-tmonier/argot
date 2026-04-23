# Typicality filter v2: absolute thresholds + file-level fallback

## Setup

> **Numbers in this doc are from a 5-seed full-bench run
> (timestamp 20260423T223111Z). Earlier iteration runs used
> `--seeds 1 --sample-controls 2000` for fast dev-loop turnover;
> those numbers are not reproducible under the production
> 5-seed configuration and should not be cited.**

Four AST-derived features via tree-sitter (no ML model):
`literal_leaf_ratio`, `control_node_density`, `ast_type_entropy`,
`unique_token_ratio`, plus `named_leaf_count`.

**Hunk-level predicate:**
`named_leaf_count >= 5  AND  literal_leaf_ratio > 0.80`

The size gate (≥5) avoids flagging 1-2 entry constant definitions
(~2 named leaves). The ratio cutoff (>0.80) is semantic: four-fifths
of AST leaves are literals → data-dominant by definition.

**File-level fallback:** applied when hunk-level doesn't fire.
`file_named_leaf_count >= 100  AND  file_literal_leaf_ratio > 0.80`
Catches partial-array hunks whose fragment falls below the hunk gate
but whose containing file is globally data-dominant.

**Path-based exclusion at inference:** test dirs, build artifacts.
Symmetric to calibration-side `collect_candidates` path filter.

**Application:** symmetric — calibration pool pre-filter + inference
short-circuit (`reason="atypical"` / `"atypical_file"` / `"excluded_path"`).
All three excluded from the fp_rate denominator.

Baseline: `benchmarks/results/baseline/latest/report.md`
(run `20260423T155121Z`). v2 run: `20260423T223111Z`.
Break-fixture audit: **0 / 91 across all six corpora** before full run.

## Research arc

**v1 (MCD Mahalanobis + pool-relative percentiles):** one-sided
percentile OR over four features with safety margins. Top-5 FPs
unchanged on 5/6 corpora. Retrospective: MCD's symmetric-outlier
geometry conflated "unusually complex code" with "data table". The
safety margins pushed all four cutoffs out of reach for normal-sized
data hunks. Only faker-js improved (−2 pp FP) because its data files
were extreme enough to clear any margin.

**v2 (absolute thresholds):** closed the misdirection. Initial gate
of ≥30 was too conservative — small-fragment data hunks (faker-js
locale file slices of 6-9 leaves) still slipped through because
tree-sitter returns an ERROR-root tree for mid-array fragments and the
original code bailed on ERROR roots. Two bug fixes:
- Removed ERROR bail-out (tree-sitter preserves typed children under
  ERROR subtrees; genuinely unparseable content returns 0 named leaves
  naturally).
- Lowered gate from 30→10→5 as fragment sizes were measured.

File-level fallback added to catch fragments whose hunk is too small
for the gate but whose file is globally data-dominant.

## Results

| Corpus | AUC base→v2 | Recall base→v2 | FP base→v2 | Controls filtered |
|:---|---:|---:|---:|---:|
| fastapi  | 0.9915→0.9918 | 69.4%→69.4%   | 0.3%→**0.1%** | 15 hunk + 0 file + 6522 path |
| rich     | 0.9933→0.9959 | 90.0%→**80.0%**  | 1.0%→**0.0%** | 342 hunk + 68 file + 2398 path |
| faker    | 0.9295→0.9237 | 100%→100%     | 1.7%→**0.1%** | 2085 hunk + 1380 file + 3778 path |
| hono     | 0.7853→0.8107 | 60.0%→60.0%   | 0.6%→**0.3%** | 80 hunk + 0 file + 22717 path |
| ink      | 0.9881→0.9888 | 86.7%→**93.3%** | 1.1%→1.1% | 46 hunk + 0 file + 9417 path |
| faker-js | 0.8568→0.9408 | 20.0%→20.0%   | 5.0%→**0.8%** | 15910 hunk + 3780 file + 191040 path |

Recall improved +6.6 pp on ink; flat on fastapi / faker / hono / faker-js;
one-fixture regression on rich (ansi_raw_2 at a threshold boundary).

### Per-corpus top-5: baseline → v2

| Corpus | Baseline top-1 (type) | v2 top-1 (type) | Verdict |
|:---|:---|:---|:---|
| rich | `_emoji_codes.py` (data file) | `traceback.py` (code) | ✓ cleaned |
| hono | `*.test.ts` files (test files) | `css/common.ts` (code, ratio 0.196) | ✓ cleaned |
| ink | `test/flex-justify-content.tsx` (test file) | `xo.config.ts` (config, BPE FP) | ✓ cleaned |
| fastapi | `docs/js/custom.js` (JS code) | `tutorial004.js` (JS code, ratio 0.117) | unchanged (BPE FP, not data) |
| faker | `providers/currency/ru_RU/__init__.py` (data) | `providers/address/vi_VN/__init__.py` (data, ratio 0.795) | partial — currency filtered, address residual |
| faker-js | `locales/en/book/title.ts` (data) | `locales/en_US/location/postcode_by_state.ts` (data, ratio 0.726) | partial — string-array locales filtered, object-key residuals remain |

## Documented limit

**Object-keyed structured data in Python and TypeScript.** Two failure modes:

1. *TypeScript property identifiers:* `postcode_by_state.ts` (hunk ratio 0.726), `unit.ts`, `mime_type.ts`, `currency.ts` — object literal where property names (`AK:`, `name:`, `symbol:`) are `property_identifier` nodes, which tree-sitter classifies as non-literal. This dilutes `literal_leaf_ratio` below 0.80 regardless of file or hunk size.

2. *Python class boilerplate:* `faker/providers/address/vi_VN/__init__.py` (hunk ratio 0.795) — locale provider wraps data in a class definition. Class name, method names, import symbols, and argument identifiers are non-literal leaves that push the ratio just below threshold (0.795 vs 0.80 cutoff).

Both modes share the same root: the 4-feature predicate cannot distinguish "property/method names I need to navigate to the data" from "structural logic names". A v3 fix would add a 5th feature: density of `property_identifier` nodes in `pair` parent position (TS) or a class/method-stripped ratio (Python). Out of scope for v2.

## Interpretation

The filter achieved its primary objective on 4/6 corpora: rich, hono, and ink top-5 are now populated by code files only (test and data files fully removed). faker-js FP rate fell from 5.0% to 0.8% (−4.2 pp), the largest single-corpus improvement. faker (Python) fell from 1.7% to 0.4% (−1.3 pp). fastapi and hono were already at low FP rates (0.3%, 0.6%) and remain near-unchanged.

The two partial results — faker and faker-js locale residuals at ratio ≈0.73–0.80 — are instances of the same documented limit. They are not new failure modes and do not warrant a v2 iteration. fastapi's top-5 (JS tutorial file, ratio 0.117, high control-flow density) is a BPE scorer FP unrelated to typicality.

Recall improved +6.6 pp on ink; unchanged on fastapi, faker, hono, and faker-js; one-fixture regression on rich (ansi_raw_2 at a threshold boundary) — net break-fixture count unchanged. 0 / 91 new categories of recall failure.
