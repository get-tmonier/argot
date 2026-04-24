# Call-receiver scorer — research (era 6)

## Setup

> **Numbers in this doc are from two 5-seed full-bench runs:
> primary (k=1) timestamp 20260424T054931Z and fallback (k=2)
> timestamp 20260424T060511Z. --sample-controls was NOT used.
> --seeds 1 was NOT used. Both are canonical runs.**

Stage 1.5 presence-based scorer over full-dotted callee signatures.
Extraction: walk call-expression callee's member-expression chain
inward, bottom out at a bare identifier, join with dots. Complex
chains (innermost object is a call/subscript) return None and are
skipped. Applied to Python (tree-sitter-python `call`/`attribute`)
and TypeScript (tree-sitter-typescript
`call_expression`/`new_expression`/`member_expression`).

Fit: union of all non-None callees across model-A files (after
`exclude_data_dominant` via `PythonAdapter`/`TypeScriptAdapter`).
Score: flag if count of distinct unattested callees in hunk >= k.
Primary config k=1; fallback k=2 (pre-declared).

**Shipping config: alpha=1.0, cap=5, with parse-fragment guard.**
k=1, k=2, alpha=0.5, and alpha=1.0 (without fix) all fail gates. After
diagnosing the root cause (see investigation phase below) and applying a
targeted fix, alpha=1.0 with the guard passes all four gates.
Timestamp 20260424T074736Z.

Baseline: `benchmarks/results/baseline/latest/report.md` (era 5,
run 20260423T231552Z).

## Research arc

Era-5 left three recall gaps visible in the baseline top-N misses:
foreign receiver calls (faker-js foreign_rng, http_sink,
runtime_fetch), framework-swap patterns in hono (express.Router,
Express middleware signatures), and Flask-style @app.route in a
FastAPI app. All share the same structural property: the hunk calls
something the repo itself never calls.

Design constrained to: no learned models, no per-framework rules,
no keyword enumeration, statistics-only derived from the repo
itself. Presence-based predicate (single binary check per callee)
was chosen over frequency-based (threshold calibration surface) on
the grounds that the misses are **absent** from the repo, not just
**rare** — a set-membership test is sufficient.

Full-dotted granularity (`Math.random`, `axios.post`, `app.route`)
was chosen over root-only (`Math`, `axios`, `app`) because root-only
misses fastapi's `app.route` (where `app` is attested as a receiver
for `app.get`/`app.post`), and over method-name-only (`random`,
`post`, `route`) because method-name-only is too noisy (collides
with legitimate usages).

## Results

### k=1 (primary config, timestamp 20260424T054931Z)

| Corpus | Recall era-5→k1 | FP era-5→k1 | Δ recall |
|:---|---:|---:|---:|
| fastapi  | 69.4%→96.3% | 0.1%→4.0%  | +26.9pp |
| rich     | 90.0%→100.0% | 0.2%→13.3% | +10.0pp |
| faker    | 100.0%→100.0% | 0.3%→3.8%  | 0pp |
| hono     | 60.0%→86.7% | 0.4%→5.2%  | +26.7pp |
| ink      | 93.3%→93.3% | 1.1%→7.6%  | 0pp |
| faker-js | 20.0%→53.3% | 0.8%→13.6% | +33.3pp |

**Gate evaluation (k=1):**

| # | Gate | Threshold | Observed | Verdict |
|---|---|---|---|---|
| 1 | Avg recall 6 corpora | ≥ 80.0% | 88.3% | PASS |
| 2 | No corpus regression > 2pp | ≥ −2 pp | min +0pp (ink, faker) | PASS |
| 3 | All corpora FP ≤ 1.5% | ≤ 1.5% | max 13.6% (faker-js) | **FAIL** |
| 4 | Category regressions from 100% | 0 / 91 | 0 | PASS |

k=1 fails Gate 3 severely — all 6 corpora exceed the 1.5% ceiling.

### k=2 (fallback config, timestamp 20260424T060511Z)

| Corpus | Recall era-5→k2 | FP era-5→k2 | Δ recall |
|:---|---:|---:|---:|
| fastapi  | 69.4%→96.3% | 0.1%→0.5%  | +26.9pp |
| rich     | 90.0%→100.0% | 0.2%→6.6%  | +10.0pp |
| faker    | 100.0%→100.0% | 0.3%→1.1%  | 0pp |
| hono     | 60.0%→66.7% | 0.4%→1.4%  | +6.7pp |
| ink      | 93.3%→100.0% | 1.1%→4.8%  | +6.7pp |
| faker-js | 20.0%→40.0% | 0.8%→5.0%  | +20.0pp |

**Gate evaluation (k=2):**

| # | Gate | Threshold | Observed | Verdict |
|---|---|---|---|---|
| 1 | Avg recall 6 corpora | ≥ 80.0% | 83.8% | PASS |
| 2 | No corpus regression > 2pp | ≥ −2 pp | min +0pp (faker) | PASS |
| 3 | All corpora FP ≤ 1.5% | ≤ 1.5% | rich 6.6%, ink 4.8%, faker-js 5.0% | **FAIL** |
| 4 | Category regressions from 100% | 0 / 91 | 0 | PASS |

k=2 improves FP substantially (fastapi 4.0%→0.5%, hono 5.2%→1.4%,
faker 3.8%→1.1%) but rich, ink, and faker-js remain far above the
1.5% ceiling.

## Stage attribution (k=2)

| Corpus | call_receiver | import | bpe | none |
|:---|---:|---:|---:|---:|
| fastapi | 76.3%+ | — | — | — |
| rich | 40.0% | 60.0% | — | — |
| faker | 0% | 100.0% | — | — |
| hono | dominant | — | — | — |
| ink | dominant | — | — | — |
| faker-js | 53.3% | — | — | 46.7% |

call_receiver fires on nearly all newly-caught breaks: fastapi async
blocking, background tasks, framework swaps all now reason
"call_receiver" rather than "bpe". This is correct — the extractor
sees blocking I/O callers (`time.sleep`, `multiprocessing.Process`,
`executor.submit`) or foreign framework constructors
(`web.Application`, `RequestHandler`) that are absent from the
FastAPI corpus.

## Root cause of FP gate failure

The fundamental problem: the attested set is built from **model-A
source files** (non-data-dominant), but real-PR control hunks come
from the **full repo** including test files. Test files routinely
contain 2+ calls to test-framework methods (`expect`, `describe`,
`beforeEach`, `afterAll`, `jest.fn`, `assertRaises`, `mock.patch`)
that never appear in the main source. At k=2, a test hunk with
`expect(x).toBe(y)` and `afterAll(() => cleanup())` fires because
both `expect.toBe` (or `expect`) and `afterAll` are absent from the
model-A source.

Evidence: rich (13.3% FP at k=1 → 6.6% at k=2) and ink (7.6%→4.8%)
have large test suites. faker-js (13.6%→5.0%) is a huge TypeScript
codebase where provider tests use Jest heavily. The corpora where
FP is acceptable at k=2 — fastapi (0.5%), faker (1.1%), hono
(1.4%) — either have smaller test surface or test patterns more
similar to source.

## Documented limits

- **FP gate failure (structural):** Presence-only check without
  distinguishing test hunks from source hunks causes false positives
  on legitimate test code. This is not tunable with k alone.
- **faker-js single-callee breaks still missed at k=2:**
  `foreign_rng_1` and `foreign_rng_3` (both `Math.random()` only)
  have one unattested callee — k=2 cannot fire.
- **Complex-chain callees still return None:**
  `hono_routing_2` (`Router().route(path).get()`) and
  `hono_routing_3` (`app.all('*', ...)`) are not caught by the
  presence check because the chain bottoms out at a call node.
- **`hono_middleware_3`** (`next()` called synchronously instead of
  awaited) — no foreign callee at all, presence check irrelevant.

## Interpretation

The extraction rule is correct and the recall story is strong: k=2
delivers +26.9pp on fastapi, +20.0pp on faker-js, +6.7pp on hono
and ink — all with zero category regressions. The call-receiver
concept works as a break detector for foreign-callee patterns.

The FP blocker is architectural: the attested set must also cover
test-file vocabulary to avoid flagging test hunks. The fix is not a
new k value — it is excluding test hunks from scoring, or extending
the attested set to include test files, or a hybrid approach. Both
are clean design changes appropriate for era 7.

Gates 1, 2, and 4 pass for both k=1 and k=2. Only Gate 3 blocks.
Era 6 lays the full research groundwork; era 7 should target the
test-file FP problem specifically.

> **Retrospective note (post-shipping).** The "test files" hypothesis in
> the paragraph above turned out to be wrong — `is_excluded_path`
> already filters test hunks before they reach the FP denominator. The
> actual root cause (tree-sitter parsing hunk fragments without their
> syntactic container) was not visible until the Investigation phase
> below. See [call-receiver-parse-fragment.md](call-receiver-parse-fragment.md)
> for the landed story.

---

## Soft penalty variant (pivot)

### Setup

Formula: `adjusted_bpe = raw_bpe + alpha * min(n_unattested, C)` where
`n_unattested` is the count of distinct unattested callees in the hunk
(same extraction rule as k-based approach), `C = 5` (cap), and
`alpha ∈ {0.5, 1.0}` (pre-declared sweep; 0.3 not needed as Gate 1
failed, not Gate 3, so fallback went to 1.0).

Flag condition: `adjusted_bpe > bpe_threshold` (threshold unchanged from
era-5 calibration — invariant because calibration hunks have n_unattested=0).

Reason attribution: `"call_receiver"` if `raw_bpe ≤ bpe_threshold` (penalty
tipped it); `"bpe"` if `raw_bpe > bpe_threshold` (BPE alone sufficient).

Timestamps: primary (alpha=0.5) `20260424T071700Z`; fallback (alpha=1.0)
`20260424T072403Z`.

**Shipping config: neither.** alpha=0.5 fails Gate 1 (recall 77.1%).
alpha=1.0 fails Gate 3 (rich 2.8%, ink 2.2% FP). Era 6 does not ship.

### Results — alpha = 0.5 (primary)

| Corpus | Recall era-5→α0.5 | FP era-5→α0.5 | Δ recall |
|:---|---:|---:|---:|
| fastapi  | 69.4%→92.6% | 0.1%→0.2%  | +23.2pp |
| rich     | 90.0%→90.0% | 0.2%→0.6%  | 0pp |
| faker    | 100.0%→100.0% | 0.3%→0.3% | 0pp |
| hono     | 60.0%→60.0% | 0.4%→0.6%  | 0pp |
| ink      | 93.3%→93.3% | 1.1%→1.5%  | 0pp |
| faker-js | 20.0%→26.7% | 0.8%→0.9%  | +6.7pp |

**Gate evaluation (alpha=0.5):**

| # | Gate | Threshold | Observed | Verdict |
|---|---|---|---|---|
| 1 | Avg recall 6 corpora | ≥ 80.0% | 77.1% | **FAIL** |
| 2 | No corpus regression > 2pp | ≥ −2 pp | min +0pp (rich, faker, hono, ink) | PASS |
| 3 | All corpora FP ≤ 1.5% | ≤ 1.5% | max 1.5% (ink) | PASS |
| 4 | Category regressions from 100% | 0 / 91 | 0 | PASS |

alpha=0.5 fails Gate 1 → per decision tree, run alpha=1.0 fallback.

### Results — alpha = 1.0 (fallback)

| Corpus | Recall era-5→α1.0 | FP era-5→α1.0 | Δ recall |
|:---|---:|---:|---:|
| fastapi  | 69.4%→94.4% | 0.1%→0.3%  | +25.0pp |
| rich     | 90.0%→100.0% | 0.2%→2.8% | +10.0pp |
| faker    | 100.0%→100.0% | 0.3%→0.6% | 0pp |
| hono     | 60.0%→60.0% | 0.4%→0.8%  | 0pp |
| ink      | 93.3%→100.0% | 1.1%→2.2%  | +6.7pp |
| faker-js | 20.0%→33.3% | 0.8%→1.3%  | +13.3pp |

**Gate evaluation (alpha=1.0):**

| # | Gate | Threshold | Observed | Verdict |
|---|---|---|---|---|
| 1 | Avg recall 6 corpora | ≥ 80.0% | 81.3% | PASS |
| 2 | No corpus regression > 2pp | ≥ −2 pp | min +0pp (faker, hono) | PASS |
| 3 | All corpora FP ≤ 1.5% | ≤ 1.5% | rich 2.8%, ink 2.2% | **FAIL** |
| 4 | Category regressions from 100% | 0 / 91 | 0 | PASS |

alpha=1.0 fails Gate 3. Both pre-declared configs fail. Era 6 does not ship.

### Shipping config

Era 6 does not ship. Neither alpha=0.5 nor alpha=1.0 clears all four gates.

### Interpretation

The soft-penalty pivot was designed to solve the test-file FP problem that
blocked k=1 and k=2: instead of flagging unconditionally on the first
unattested callee, a fractional penalty (alpha * n) would leave most
single-unattested-callee control hunks below threshold. At alpha=0.5, this
works well for FP — Gate 3 clears with room to spare (max 1.5% at ink,
versus 13.6% at k=1). But the penalty is also too small to push the low-BPE
foreign-callee breaks (hono framework-swap at BPE 1.484, faker-js
foreign_rng at BPE 0.52) over threshold. The result: hono stays at 60%
recall (era-5 baseline) and faker-js recovers only modestly to 26.7%,
dragging the average to 77.1% — below the 80.0% floor.

At alpha=1.0, recall climbs meaningfully (fastapi 94.4%, ink 100%, rich
100%, faker-js 33.3%, avg 81.3%) and Gate 1 clears. But the larger penalty
triggers on test-file calls again: rich rises to 2.8% FP and ink to 2.2%,
both breaching the 1.5% Gate 3 ceiling. This is the same structural blocker
as k=2: test hunks with 2+ framework-method calls (Jest matchers, React
lifecycle methods) accumulate enough penalty to cross threshold.

The root cause is unchanged across all era-6 variants: the attested set is
built from model-A source files, but control hunks are drawn from the full
repo including test files. Test files use framework methods (Jest, React
Testing Library, pytest) that never appear in production source. Until the
attested set includes test-file callees, any alpha that catches low-BPE
foreign breaks will also penalize legitimate test code. Era 7 must solve
this asymmetry before the call-receiver concept can ship.

---

## Investigation phase (post-alpha-sweep)

> Timestamp: 20260424T074736Z. No new bench until root cause verified.
> Previous evidence blamed "test files" without verifying — incorrect.
> `is_excluded_path` already removes test hunks from the FP denominator.

### Phase 1: FP path-category breakdown

Script: one-off `/tmp/investigate_fps.py` (not committed).
Data source: `benchmarks/results/20260424T072403Z/` (alpha=1.0 run).

#### rich — call_receiver FPs (225 of 244 total control FPs)

| Path segment | Count | Sample files |
|:------------|------:|:-------------|
| `rich/`     |   225 | rich/traceback.py (35), rich/table.py (30), rich/console.py (25), rich/panel.py (22), … |

All 225 call_receiver FPs are from **core library source files** (`rich/*.py`).
No test files, no docs, no examples.

#### rich — top unattested callees (re-derived via scorer replay)

```
theme                  23    locals_max_string      22
console                21    title                  21
detect.height          21    locals_max_depth       18
cells                  18    width                  16
fonts.Defaults.padding 16    to                     16
```

These are **not function names** — they are Google-style docstring parameter
descriptions: `param_name (type): description`. Tree-sitter, receiving the
hunk slice in isolation (outside its enclosing triple-quote), parses
`param_name(type)` as a `call` expression with `param_name` as the callee.

#### ink — call_receiver FPs (83 of 160 total control FPs)

| Path segment | Count | Sample files |
|:------------|------:|:-------------|
| `src/`      |    83 | src/reconciler.ts (most), src/ink.tsx, src/render.ts |

All 83 call_receiver FPs from **core ink source files**.

#### ink — top unattested callees

```
removeChildFromContainer  18    commitUpdate            14
commitTextUpdate          12    prepareScopeUpdate       9
getInstanceFromScope       9    removeChild              8
setCurrentUpdatePriority   8    …
```

These are React reconciler host-config **method shorthand definitions**
(`commitTextUpdate(node, _oldText, newText) { … }`). Extracted out of the
enclosing `createReconciler({…})` object literal, tree-sitter parses each
method shorthand as a `call_expression` followed by a block statement.

### Root cause

Both patterns share the same failure mode: **tree-sitter receives a hunk
slice that is an out-of-context fragment**.

When the benchmark extracts `lines[hs:he]` from the current file and passes
that substring to `extract_callees`, tree-sitter parses it as standalone
Python/TypeScript. If those lines are inside a triple-quoted docstring (rich)
or inside an object literal (ink reconciler), the resulting parse tree has
root-level ERROR nodes — and the false "call expressions" inside the fragment
produce callee strings that never appear in the attested set.

**Key indicator**: in both cases, tree-sitter reports root-level ERROR nodes
when parsing the hunk content in isolation:

| Corpus | call_receiver FPs | Have root ERROR nodes | Fraction |
|:-------|------------------:|----------------------:|---------:|
| rich   |               225 |                   202 |     89.8% |
| ink    |                83 |                    71 |     85.5% |

### Classification

**Targeted parsing artifact** (closest to Case A): FPs are concentrated in a
specific hunk-content type — fragments that parse with root-level ERROR nodes —
not in a path category, not in "new helper" sensitivity. The fix is a content
guard in the bench harness, not a path-filter change.

### Fix applied

`benchmarks/src/argot_bench/call_receiver.py`:

1. Added `_has_root_error(source, language) → bool`: parses the hunk and
   returns True if any direct child of the root node is an ERROR node.
2. In `_get_distinct_unattested`: return `[]` immediately if
   `_has_root_error(hunk_content, language)`. This treats parse-fragment
   hunks as having zero unattested callees → zero soft penalty → no
   call_receiver flag.
3. Break fixtures are complete code sections; they parse cleanly and are
   unaffected.

Projected impact (verified via scorer replay on alpha=1.0 JSON):

| Corpus | FP before fix | FP projected | Passes Gate 3 (≤1.5%)? |
|:-------|-------------:|--------------:|:-----------------------|
| rich   |         2.8% |         0.36% | ✓                      |
| ink    |         2.2% |         0.49% | ✓                      |

### Bench results: alpha=1.0 + parse-fragment guard (20260424T074736Z)

Full 5-seed, all 6 corpora, no sample-controls.

| Corpus | Lang | AUC | Recall | FP | Gap | N_fix | N_ctrl | Thr |
|:---|:---|---:|---:|---:|---:|---:|---:|---:|
| fastapi | python | 0.9918 | 91.7% | 0.1% | -4.371 | 31 | 10012 | 5.278 |
| rich | python | 0.9959 | 100.0% | 0.5% | -2.017 | 10 | 11536 | 4.164 |
| faker | python | 0.9237 | 100.0% | 0.4% | -4.946 | 5 | 12936 | 5.211 |
| hono | typescript | 0.8107 | 60.0% | 0.4% | -7.471 | 15 | 54717 | 4.277 |
| ink | typescript | 0.9888 | 100.0% | 1.1% | -4.633 | 15 | 16678 | 4.743 |
| faker-js | typescript | 0.9408 | 33.3% | 0.8% | -7.066 | 15 | 255760 | 4.773 |

#### Gate evaluation

| # | Gate | Threshold | Observed | Verdict |
|---|---|---|---|---|
| 1 | Avg recall 6 corpora | ≥ 80.0% | 80.8% | **PASS** |
| 2 | No corpus regression > 2pp | ≥ −2 pp | min +0.0pp (faker, hono) | **PASS** |
| 3 | All corpora FP ≤ 1.5% | ≤ 1.5% | max 1.08% (ink) | **PASS** |
| 4 | Category regressions from era-5 100% | 0 | 0 | **PASS** |

**Shipping config: alpha=1.0, cap=5, `_has_root_error` guard.**
All four gates pass. Era 6 ships.

#### Notes on recall changes vs alpha=1.0 (no fix)

- fastapi 94.4% → 91.7%: `validation_2` (voluptuous) was caught by
  call_receiver only when its hunk had parse errors. This fixture was NOT
  caught in era-5 baseline (75.0% validation era-5). Gate 4 does not fire.
- All other corpora: recall unchanged vs prior alpha=1.0 run.

---

## Era 9 — alpha=2.0 sweep (20260424T163221Z)

### Setup

Hypothesis: faker-js `http_sink_1` (BPE 3.281, 1 unattested callee) and
`http_sink_3` (BPE 2.841, 1 unattested callee) cross threshold at α≥2
(adj 5.281 and 4.841 vs threshold 4.773). Primary α=3.0; fallback α=2.0.

α=3.0 run failed Gate 3: faker FP 1.6% > 1.5%. Per pre-registered decision
tree, fallback α=2.0 was run. Full 5-seed bench, all 6 corpora.

**Shipping config: alpha=2.0, cap=5.**

### Results (alpha=2.0)

| Corpus | Lang | AUC | Recall | FP | Gap | N_fix | N_ctrl | Thr |
|:---|:---|---:|---:|---:|---:|---:|---:|---:|
| fastapi | python | 0.9880 | 91.7% | 0.8% | -4.371 | 32 | 79623 | 5.278 |
| rich | python | 0.9780 | 95.0% | 0.8% | -3.040 | 16 | 68598 | 4.164 |
| faker | python | 0.9537 | 95.0% | 1.2% | -4.946 | 16 | 75996 | 5.211 |
| hono | typescript | 0.8312 | 78.3% | 0.5% | -7.471 | 17 | 54717 | 4.277 |
| ink | typescript | 0.9899 | 93.3% | 0.4% | -4.633 | 17 | 16678 | 4.826 |
| faker-js | typescript | 0.9463 | 53.3% | 1.0% | -7.066 | 17 | 255760 | 4.773 |

Avg recall: 84.4% (+3.8 pp vs era-8 baseline of 80.57%).

### Gate evaluation

| # | Gate | Threshold | Observed | Verdict |
|---|---|---|---|---|
| 1 | Avg recall ≥ 83.5% | 83.5% | 84.4% | **PASS** |
| 2 | No corpus regression > 1 fixture | — | all improved | **PASS** |
| 3 | All corpora FP ≤ 1.5% | 1.5% | max 1.2% (faker) | **PASS** |
| 4 | 0 category regressions from 100% | 0 | 0 | **PASS** |
| 5 | faker-js ≥ 9/17 | 9/17 | 9/17 | **PASS** |
| 6 | hono ≥ 12/17 | 12/17 | 13/17 | **PASS** |

All 6 gates pass. Era 9 ships.

### Fixtures moved

| Fixture | Old label | New label |
|---|---|---|
| faker_js_http_sink_1 | uncaught | hard |
| faker_js_http_sink_3 | uncaught | hard |
| hono_routing_3 | uncaught | hard |
| ink_dom_access_1 | uncaught | hard |
