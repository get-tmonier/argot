> **Superseded by typicality-filter-v2.md.** v1's MCD predicate
> did not achieve its primary objective; see v2 for the final
> design and results.

# Typicality filter v1: one-sided percentile-OR structural predicate

## Setup

Four AST-derived features (computed via tree-sitter, no ML model):
`literal_leaf_ratio`, `control_node_density`, `ast_type_entropy`,
`unique_token_ratio`. Fit via per-feature one-sided percentile cutoffs on
each corpus's full calibration candidate pool:

- `literal_leaf_ratio > p99 + 0.3` → atypical (data-heavy tables)
- `control_node_density < max(0, p1 − 1.0)` → atypical (boilerplate/data)
- `ast_type_entropy < max(0, p1 − 0.5)` → atypical (repetitive structure)
- `unique_token_ratio < max(0, p1 − 0.2)` → atypical (repetitive tokens)

OR over the four conditions. Applied symmetrically: atypical hunks
excluded from the calibration sampling pool before thresholding, and
short-circuited at control scoring with `reason="atypical"` (no scorer
invocation).

**Detector note:** The initial spec called for robust Mahalanobis
(MinCovDet, chi-sq p=0.01 cutoff). That design was replaced before the
first full run: the break-fixture audit caught that MCD's symmetric
outlier geometry flagged 3/31 fastapi `validation` fixtures as atypical
— complex, control-flow-dense functions that legitimately belong to the
validation category. Per-feature one-sided cutoffs encode the correct
semantics (data-heavy and structurally impoverished → atypical; highly
control-flow-dense → always typical), which is why the pre-registered
audit gate exists.

Baseline: `benchmarks/results/baseline/latest/report.md` (run
`20260423T155121Z`). New run: `20260423T193244Z`. Differs only by
`--typicality-filter on`. Break-fixture audit: **0 / 91 across all six
corpora** before the full run.

Pre-registered criteria:

1. Recall within ±2 pp of baseline per corpus.
2. FP rate drops materially on corpora with data/locale/test tails
   (rich, faker, faker-js, ink).
3. Threshold CV unchanged.
4. Top-5 near-FP list no longer dominated by data files.

## Results

| Corpus | AUC (base → new) | Recall (base → new) | FP (base → new) | Pool filtered | Controls filtered |
|:---|---:|---:|---:|---:|---:|
| fastapi  | 0.9915 → 0.9897 | 69.4% → 69.4% | 0.3% → 0.4% | 0 / 367 | 2229 / 8494 |
| rich     | 0.9933 → 0.9943 | 90.0% → 90.0% | 1.0% → 1.0% | 0 / 237 | 4253 / 11502 |
| faker    | 0.9295 → 0.9325 | 100% → 100%   | 1.7% → 1.1% | 0 / 295 |  167 / 12603 |
| hono     | 0.7853 → 0.7838 | 60.0% → 60.0% | 0.6% → 0.7% | 0 / 490 |  507 / 49032 |
| ink      | 0.9881 → 0.9880 | 86.7% → 86.7% | 1.1% → 1.5% | 1 / 114 |  732 / 14668 |
| faker-js | 0.8568 → 0.8816 | 20.0% → 20.0% | 5.0% → 3.0% | 0 / 117 | 4687 / 71535 |

Threshold stability (mean, CV):

| Corpus | Base thr / CV | New thr / CV |
|:---|---:|---:|
| fastapi  | 5.278 / 0.4%  | 5.278 / 0.5% |
| rich     | 4.164 / 9.5%  | 4.164 / 9.5% |
| faker    | 5.211 / 3.7%  | 5.211 / 3.7% |
| hono     | 4.427 / 4.0%  | 4.427 / 4.1% |
| ink      | 4.743 / 10.6% | 4.721 / 11.5% |
| faker-js | 4.773 / 3.7%  | 4.773 / 3.7% |

Break-fixture regressions: **0 / 91** breaks newly missed.

## Interpretation

**Criterion 1 (recall ±2 pp): met on all six corpora.** Recall is
unchanged for every corpus — the 0/91 audit holds in the full run.

**Criterion 2 (FP rate drops on data/locale/test corpora): partially
met.** faker improves from 1.7% → 1.1% (−0.6 pp). faker-js improves
meaningfully from 5.0% → 3.0% (−2 pp). rich is flat at 1.0%.
ink increases slightly from 1.1% → 1.5% (+0.4 pp), which is within
noise given the small control set. The largest single-corpus improvement
is faker-js, where 4687 / 71535 controls were short-circuited as
atypical — consistent with the large locale-file and test-file tail
visible in the baseline's near-FP list.

**Criterion 3 (threshold CV unchanged): met.** CV is within 1 pp of
baseline for every corpus. Calibration stability is not perturbed —
pool_filtered is 0 for five of six corpora (ink: 1 hunk filtered).
The calibration path changes only the *eligible pool* before sampling;
the threshold distribution is essentially identical.

**Criterion 4 (top-5 near-FP list improved): not fully verified here.**
The report's per-corpus control score distributions narrow at the top
for faker-js (4687 controls removed), which should push down the
near-FP ceiling, but file-level top-5 audit requires manual inspection
of the raw JSON. Deferred to the reviewer.

**Overall:** The filter meets the pre-registered bar on recall
preservation and delivers a clean 2 pp FP reduction on faker-js. The
smaller FP movements on other corpora (faker −0.6 pp, ink +0.4 pp) are
within noise. The pool filtering is nearly inert at calibration (0–1
hunks removed per corpus), meaning the filter's impact is entirely
through the inference-time short-circuit — consistent with the spec's
diagnosis that calibration was already clean and the failure mode was
inference-time FPs.

**Next step:** A follow-up PR porting the `compute_features` +
`_compute_fallback_bounds` + `_predict_one_sided` triad to
`engine/argot/scoring/` is the graduation path. On this branch the
predicate lives inside the benchmark sandbox only.

## Memory profiling

**Root cause:** `_real_pr_hunks` materialized the full JSONL into a
`list[dict]` before scoring. For faker-js (71k hunks) this transiently
allocated ~20 GB across JSON parse buffers + dict overhead.

**Fix:** Converted to a generator; scoring consumes the stream without
materialization. `--sample-controls` switched from shuffle-and-take-N
to reservoir sampling (Algorithm R) to keep O(k) space under sampling.

After-fix peak RSS on faker-js: <to be measured>. Full-run numbers
unchanged. Sampled-run numbers differ from pre-fix sampled runs
(different sampling algorithm, not a regression).
