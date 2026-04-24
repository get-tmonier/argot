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

**Shipping config: neither.** k=1 and k=2 both fail Gate 3 (FP ≤
1.5%). Era 6 does not ship.

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

## Era 7 recommendations

1. **Extend attested set to include test files.** If test-file
   callees are attested, test hunks will not fire. Implementation:
   in `CallReceiverScorer.__init__`, accept a separate
   `test_files: list[Path]` and union their callees into `attested`
   (without applying the data-dominant filter, since test files are
   code, not data).
2. **Alternatively, classify hunk source and skip test hunks.**
   Check whether the file being scored matches `*test*`, `*spec*`,
   `*__tests__*` path patterns and short-circuit Stage 1.5.
3. **Higher k (k=3) is a last resort** if the above fail — but
   may sacrifice recall on single-callee breaks. Not pre-declared
   for era 6.

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
