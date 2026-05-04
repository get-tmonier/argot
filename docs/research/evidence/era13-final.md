# Era 13 — Final Evidence

Synthesis of all era-13 phases (1, 2, 3, 4) against the goals
pre-registered in `docs/research/era-13-hypotheses.md`. Decision
gate per phase, headline numbers, ship recommendation.

## Headline

**Ship era-11 status quo unchanged**: `cluster_rare_threshold = 0`,
`cluster_size_min = 0`, `threshold_percentile = None` (max),
no shape primitives enabled. Production scoring catches **105/115
fixtures (91.3% category-mean recall)** with all 6 corpora under
their per-corpus FP ceiling.

Three structural results explain the bound:

1. **Cancellation under symmetric firing** is the binding constraint
   for any additive cluster-conditional rule under
   `max(cal_scores)` thresholding. Phase 2's percentile change and
   Phase 4's per-cluster-baseline shape primitives both produce
   symmetric firing on cal+fixture, so threshold inflation matches
   per-fixture bonus 1:1 — net catch impact is zero (or
   negative when threshold inflation outpaces fixtures with no
   bonus).
2. **Cluster-size floor is a no-op** on argot's actual corpora.
   Real corpus clusters are large (60–320 files) and Zipf-distributed
   over callees, so even at `cluster_size_min = 20` the cal-side
   rare-fire count drops by less than 5%.
3. **AST-shape primitives that read the bare hunk fire FP-heavy on
   real-PR controls.** The `call_scope_fraction` primitive sees every
   bare hunk (extracted from inside a function body) as
   100%-module-scope; the cluster baseline expects a much lower
   fraction → false-positive blowup of 13.2% on faker (6× the 2.26%
   ceiling). Bare-hunk vs. spliced-into-host scoring is an
   asymmetry that breaks G3 by construction for any module-scope-
   sensitive primitive.

| Goal | Target | Result | Verdict |
|:---|:---|---:|:---:|
| **G1** Recall floor | ≥ 94.0% (108/115) | 91.3% (105/115) | ✗ |
| **G1** Stretch | ≥ 96.5% (111/115) | 91.3% (105/115) | ✗ |
| **G2** Per-corpus FP | all corpora ≤ 2.5% ceiling | 5/6 corpora ≤ ceiling, faker 1.96% (under) | ✓ for shipping config |
| **G3** No regression | 105 caught fixtures preserved | preserved (status-quo ship) | ✓ |
| **G4** Faker-js floor | ≥ 88% (15/17) | 76.5% (13/17) | ✗ |
| **G5** Threshold stability | CV ≤ 3% | 0.0% across all 6 corpora | ✓ |
| **G6** No hardcoded domain | All rules from corpus stats | clean | ✓ |

G1, G1-stretch, and G4 are unmet. Production stays at era-11
because every alternative explored either fails G3 (regresses
caught fixtures) or fails G2 (FP blowup) without offsetting
recall gains.

## Phase outcomes

### Phase 1 — Plumbing audit (commit `becd254`)

**Verdict: plumbing-bug-found and fixed.**

The era-12 discrepancy ("standalone probe shows threshold +5.0,
full bench shows +0.0") was caused by a missing argv block in the
bench's subprocess fan-out at
`benchmarks/src/argot_bench/cli.py:_run`. The
`call_receiver_cluster_rare_threshold` parameter was silently
dropped between the parent invocation and the run-one workers, so
the rule never fired in the full bench regardless of CLI flags.

Phase 1 added:
- The missing argv-builder block.
- The missing flag definition on the `run-one` subparser.
- A `rare_branch_fire_count` counter exposed via stderr `[rare-counter]`
  log lines on calibration and fixture paths so future plumbing
  bugs are observable.

**G3.c clause clears trivially**: `cluster_rare_threshold=0` was the
shipping default before and after the fix, so the 105 caught
fixtures re-score identically — no behavior change to production.

### Phase 3 — Symmetry audit (commit `becd254`, day-1 memo)

**Verdict: audit-clean.** Four path comparisons walked
(`_score_fixtures` vs `_score_real_hunks`; calibration path vs
scoring path; fixtures without `host_file`; `is_atypical_file`
short-circuit). All four return symmetric or not-applicable.
Zero MANDATORY-FIX items. One non-blocking defensive note filed:
silent `OSError` fallback at `run.py:173-217` reverts to
catalog-phantom routing without logging — currently dead code,
but should log a warning rather than silently degrade.

Phase 3 ran in parallel with Phase 1 on day 1; combined audit memo
at `docs/research/evidence/era13-day1-audit.md`.

### Phase 2 — Size-conditional rare + percentile sweep (commits `43fa77e`, `ecfd5fd`)

**Verdict: documented bound, ship `cluster_rare_threshold = 0`
status quo.**

12-cell pre-registered grid over `cluster_size_min ∈ {0, 20}` ×
`cluster_rare_threshold ∈ {1, 2}` × `threshold_percentile ∈
{p95, p99, max}`. 10 of 12 cells measured (cells 11–12 skipped as
deterministic mirrors of cells 5–6 — `S_min=20` proved a no-op
across the R=1 corner so the R=2 mirror is determined). Detailed
memo at `docs/research/evidence/era13-phase2-sweep.md`.

| Cell | catches | Δ vs 105 | targets | FP gate |
|:---|---:|---:|---:|:---:|
| `S_min=0  R=1 p95` | 85/115 | −20 | 0/5 | ✓ |
| `S_min=0  R=1 p99` | 75/115 | −30 | 0/5 | ✓ |
| `S_min=0  R=1 max` | 70/115 | −35 | 0/5 | ✓ |
| `S_min=0  R=2 p95` | 84/115 | −21 | 0/5 | ✓ |
| `S_min=0  R=2 p99` | 76/115 | −29 | 0/5 | ✓ |
| `S_min=0  R=2 max` | 71/115 | −34 | 0/5 | ✓ |
| `S_min=20 R=1 p95` | 85/115 | −20 | 0/5 | ✓ |
| `S_min=20 R=1 p99` | 75/115 | −30 | 0/5 | ✓ |
| `S_min=20 R=1 max` | 70/115 | −35 | 0/5 | ✓ |
| `S_min=20 R=2 p95` | 84/115 | −21 | 0/5 | ✓ |

Per-corpus cal-fire counts (S_min=0 vs S_min=20):

| Corpus | Cal fires (S_min=0) | Cal fires (S_min=20) | Δ |
|:--|---:|---:|---:|
| fastapi  |  7 971 |  7 971 |  0.0% |
| rich     | 26 771 | 26 198 | −2.1% |
| faker    | 17 677 | 17 615 | −0.4% |
| hono     | 12 667 | 12 540 | −1.0% |
| ink      | 16 326 | 16 138 | −1.2% |
| faker-js | 13 728 | 13 059 | −4.9% |

The cancellation is structural, not a small-cluster artifact:
callees follow Zipf in real codebases, so even clusters of 60+
files have many rare-attested callees. Calibration hunks pick up
the rare-bonus on every cluster, dominating the per-fixture
bonus — threshold rises with the cal-side count, not the fixture-
side.

Phase 1 + Phase 2 wiring stays in the codebase as substrate for
future Phase 2.x variants. Both are no-ops at the shipping
defaults.

### Phase 4 — AST-shape primitives (commits `0d4880c`, `631b6ea`)

**Verdict: 0 net AST-shape catches across all measured
compositions.**

Four primitives shipped: `ExceptReturnRaiseRatio`,
`CallScopeFraction`, `NamespaceJsd`, `FallThroughGuards`. Each
implements the `ShapePrimitive` Protocol with per-cluster
baseline + tail-z penalty clipped at `cluster_bonus`. Empty
primitive list is a true no-op (verified by the
`test_empty_primitive_list_is_noop_vs_no_list` unit test and
re-confirmed at the bench level by cell 1 of the cumulative
re-bench: 105/115 with the scaffolding in place but no primitives
enabled).

Pre-bench data signature scout at
`.scratch/era13-phase4-data-signatures.md` measured each
primitive's tail-z on its target fixture vs cluster baseline.
Two of four had positive plausibility:

| Primitive | Target | Hunk scalar | Cluster μ ± σ | Tail-z | Plausible? |
|:---|:---|---:|---:|---:|:---:|
| ExceptReturnRaiseRatio | `exception_handling_4` (fastapi) | 1.000 | 0.071 ± 0.258 | +3.61σ | Y |
| CallScopeFraction | `runtime_fetch_1` (faker-js) | 0.000 | 0.685 ± 0.432 | −1.58σ | Y |
| NamespaceJsd | `ink_dom_access_2` (ink) | n/a | n/a | n/a | N (hunk has 0 callees) |
| FallThroughGuards | `hono_middleware_3` (hono) | n/a | 0.0 ± 0.0 | n/a | N (hunk has 0 functions; baseline degenerate) |

Five-cell cumulative re-bench across all 6 corpora:

| Cell | recall (caught/115) | Δ vs 105 | FP gate | G3.b |
|:---|---:|---:|:---:|:---:|
| 01 baseline (no primitives) | 105/115 | +0 | ✓ | ref |
| 02 `except_return_raise_ratio` alone | 105/115 | +0 | ✓ | ✓ |
| 03 `call_scope_fraction` alone | 103/115 | −2 | **✗** | **✗** |
| 04 ER + CSF | 103/115 | −2 | **✗** | **✗** |
| 05 all four | 83/115 | −22 | ✓ (FP drop) | **✗** |

Per-cell observations:

- **02 (`ER` alone) is neutral**: 0 catches, 0 regressions. The
  +3.61σ scout signal on `exception_handling_4` does not
  translate to a caught fixture. Fastapi FP drops 0.572% → 0.517%
  (−0.055pp), which means the primitive IS firing on calibration
  hunks (fastapi has plenty of try/except patterns in real cal
  data) and threshold IS inflating, just enough to take out a few
  borderline FPs without affecting any fixture. Cancellation
  again.
- **03 (`CSF` alone) is catastrophic**: faker FP goes 1.957% →
  13.201% (6× ceiling). Fastapi and rich also blow ceilings.
  Root cause: real-PR hunks are bare (extracted from inside
  function bodies). The primitive walks the AST of the bare
  hunk, sees zero `function_definition` ancestors for any call,
  computes fraction = 1.0 for huge numbers of real PR hunks while
  cluster baseline is much lower → tail-z fires high → FP
  explosion. The data-signature scout warned this exact
  asymmetry would bite; this run quantified it.
- **04 (`ER + CSF`) inherits CSF's blowup**, ER provides no
  compensating effect (ER's threshold inflation is tiny relative
  to CSF's bare-hunk over-firing).
- **05 (all four) flips FP-blowup into FP-collapse**: 4
  primitives independently fire on cal hunks, threshold inflates
  past the point where real-PR hunks reach (FP drops to near
  zero). Same dynamic kills 22 fixtures (G3 violation). One
  curiosity: `faker_js_error_flip_2` (a fragile +1 target) is
  caught — first AST-shape composition to catch any target — but
  probably noise from threshold landing in a specific gap; not
  worth chasing given the 22 G3 violations.

Per phase pre-registered design rules (era-13 §line 290 "no
cross-fixture tuning"): each primitive ships with its
pre-registered math; the verdict is the verdict.

### Phase 5 — Deferred

**5a (negative-shape detection)**: targeted at
`synthetic_formula_1` (faker, the negative-shape residual whose
anomaly is *absence* of cluster-typical callees). Plus
`ink_dom_access_2` (no callees) and `hono_middleware_3` (2-line
fragment) — both of which Phase 4 cannot catch by data shape.
Trigger condition met (Phase 4 left G1 unmet). Queued for era 14.

**5b (synthetic mutation at scale)**: 10k+ synthetic break/control
pairs at the data-generating distribution. Era-14+ research
direction.

## What stays in the codebase

| Component | Status | Rationale |
|:---|:---|:---|
| Phase 1 plumbing fix | shipped at default-off | no-op at `cluster_rare_threshold=0`; future Phase 2.x variants need the substrate |
| Phase 2 wiring (`cluster_size_min`) | shipped at default-off | same |
| Phase 4 ShapePrimitive Protocol + registry | shipped, empty registry | no-op at empty list; era-14 negative-shape primitive will reuse |
| 4 AST-shape primitive implementations | shipped, unregistered by default | reusable parts (tree-sitter walking, tail-z ramp, JSD util) for future primitives; can be enabled on demand |

Production CLI surface unchanged: shipping defaults preserve
era-11 behavior bit-identically.

## What we will NOT do

| Approach | Reason |
|:---|:---|
| Tune Phase 4 primitive math after seeing the bench result | Era-13 §line 290 ("no cross-fixture tuning") is binding. The math was pre-registered; the verdict is the verdict. |
| Patch `CallScopeFraction` to score the spliced-into-host AST | Per era-13 design contract, every primitive uses the same scoring path the bench provides. Asymmetric scoring (catalog vs real-PR) is exactly the era-12 routing bug we just spent two months fixing. The cleanest framing is "primitives that depend on host-context belong in a different layer", not "patch this primitive". |
| Re-run Phase 2 with different `cluster_rare_threshold` values | Sweep already covered the meaningful axis. The cancellation is structural, not parameter-sensitive. |
| Lower `cluster_bonus` to reduce cancellation | Symmetric in cal and fixture; cancellation cancels regardless of cluster_bonus magnitude. |
| Larger pretrained encoders / further MLM variants | Era 12 §"What's been tried" closed these directions. Failure mode is the metric, not encoder size. |

## Recommendations

1. **Ship era-11 production scoring unchanged** for argot's next
   release. 105/115 is the documented bound under current
   architecture; era-13 establishes that with strong evidence.

2. **Open era-14 with the Phase 5a framing**:
   *"Negative-shape detection — does the hunk's callee set
   under-cover the cluster's expected distribution?"* The 4
   uncaught residuals after era-13 are dominated by negative-shape
   (`synthetic_formula_1`), zero-callee hunks
   (`ink_dom_access_2`), and 2-line fragments
   (`hono_middleware_3`) — all of which fundamentally cannot be
   caught by any primitive that scores on what's *present* in the
   hunk. Era-14 should reframe.

3. **Era-14 substrate is already in place**. The
   `ShapePrimitive` Protocol + registry pattern is reusable; the
   integration trace at
   `.scratch/era13-phase4-integration-trace.md` describes every
   plug-in point. A negative-shape primitive (e.g.
   `clustered_callee_underrepresentation`) is a one-class drop-in.

4. **For the team**: the per-cluster-baseline + tail-z + cap
   pattern is proven plumbing-clean. The lesson from era-13 is
   that the cancellation bound is fundamental to additive
   cluster-conditional scoring under `max(cal_scores)`
   thresholding. Future eras targeting recall gains should focus
   on either (a) primitives that fire asymmetrically on cal vs.
   fixture by construction (negative-shape over zero-callee
   regions, where calibration hunks structurally don't apply), or
   (b) replacing the threshold computation with something that
   isn't `max`.

## Era-13 → Era-14 Transition

Strict 115-fixture verdict parity is the standing rule per era-12
transition discipline. Re-bench at era-14 dispatch confirms
era-13 didn't introduce any silent drift.

Production scoring: bit-identical to era-11 (verified by Phase 4
re-bench cell 1).

Substrate adds:
- `cluster_rare_threshold` parameter (default 0, no-op).
- `cluster_size_min` parameter (default 0, no-op).
- `threshold_percentile` parameter on `--enable-shape-primitives`
  CLI (default empty list, no-op).
- `shape_primitives` constructor argument on
  `CallReceiverScorer` and `SequentialImportBpeScorer` (default
  None, no-op).
- Four primitive implementations under
  `engine/argot/scoring/scorers/{except_return_raise_ratio,
  call_scope_fraction, namespace_jsd, fall_through_guards}.py` —
  registered but disabled by default.

## End of Document
