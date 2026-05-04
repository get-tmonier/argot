# Era 13.5 — Final Evidence

Synthesis of the era-13.5 scoped follow-on: Phase A (asymmetric calibration),
Phase B (negative-shape primitive), Phase C (host-context AST — dropped),
plus the **per-corpus auto-detect mechanism** that emerged from a night of
post-PRD experimentation.

Era 13.5 succeeds era 13 (`era13-final.md`, commit `062ff74`) which closed at
105/115 = 91.3% recall with the recommendation to ship era-11 status quo.

## Headline

**Ship era-13.5 with `--auto-select-asym-cal` + `--call-receiver-cluster-rare-threshold=2`**.

Production scoring catches **108/115 fixtures (93.9% fixture-count recall)**
under K=7 multi-seed calibration. All six corpora under their per-corpus FP
ceiling (max 2.0%). Net **+3 catches** (all on faker-js: `foreign_rng_1`,
`http_sink_2`, `runtime_fetch_1`); zero regressions on the other five corpora.

| Goal | Target | Result | Verdict |
|:---|:---|---:|:---:|
| **G1** Recall floor | ≥ 94.0% (108/115) | 93.9% (108/115) | borderline ✗ |
| **G1** Stretch | ≥ 96.5% (111/115) | 93.9% (108/115) | ✗ |
| **G2** Per-corpus FP | all corpora ≤ 2.5% ceiling | all 6 ≤ 2.0% | ✓ |
| **G3** No regression | 105 caught fixtures preserved | preserved + 3 new | ✓ |
| **G4** Faker-js floor | ≥ 88% (15/17) | 94.1% (16/17) | ✓ |
| **G5** Threshold stability | CV ≤ 3% | 0.0% across all 6 corpora | ✓ |
| **G6** No hardcoded domain | All rules from corpus stats | clean (auto-detect uses corpus-level callee statistics only) | ✓ |
| **G7** Calibration contract documented | Phase A asym mechanism has a written contract | `docs/agents/calibration-contract.md` | ✓ |

G1 floor missed by exactly 1 fixture (108/115 = 93.91%; floor was 108/115 =
93.91% if rounding 94.0% down to 108-fixture count, or 109/115 = 94.78% if
rounding up). **Reading G1 as "≥108 catches" → cleared exactly. Reading G1
as "≥94.0% recall" → borderline miss at 93.91%.** Single-seed runs hit
109/115 (catches `ink_dom_access_2` at threshold 3.742 vs canonical K=7
threshold 4.993), but the multi-seed median is the production setting.

The remaining 7 residuals require fundamentally new mechanisms (parse-error
blocked AST bonus on fastapi; structural anomalies outside callee/shape
framing on the others). Era 14 backlog.

## Per-corpus result (canonical K=7 bench)

Numbers from `benchmarks/results/baseline/latest/report.md` (committed
alongside this memo).

| Corpus | Auto-detect probe | Decision | Recall | FP | G2 (≤2.5%) |
|---|---|---|---|---|---|
| fastapi | fire_rate ~0.12 | DISABLE (= baseline) | 30/32 (93.8%) | 0.57% | ✓ |
| rich | fire_rate ~0.22 | DISABLE | 16/16 (100%) | 1.23% | ✓ |
| faker | fire_rate ~0.25 | DISABLE | 15/16 (93.8%) | 1.96% | ✓ |
| hono | fire_rate ~0.11 | DISABLE | 15/17 (88.2%) | 0.51% | ✓ |
| ink | fire_rate ~0.10 | DISABLE | 16/17 (94.1%) | 0.54% | ✓ |
| **faker-js** | **fire_rate ~0.02** | **KEEP rule (asym)** | **16/17 (94.1%)** | **2.00%** | **✓** |
| **TOTAL** | — | — | **108/115 = 93.9%** | all ≤ 2.0% | **6/6 ✓** |

vs era-11/era-12/era-13 baseline: 105/115 = 91.3%. Net **+3 catches** (all
faker-js). Single-seed runs sometimes catch `ink_dom_access_2` at lower
threshold (yields 109/115 = 94.8%), but the K=7 multi-seed median is the
shipping setting.

## Phase outcomes

### Phase A — Asymmetric calibration (commit `58c3a58`)

**Verdict: ships, gated by per-corpus auto-detect.**

The mechanism: the calibration path computes its threshold WITHOUT the
era-13 Phase 10 cluster_rare contribution and (when enabled) the Phase-4
ShapePrimitive contributions; the fixture/scoring path fires all
contributions additively. Implemented as
`apply_optional_contributions_to_cal: bool = False` on `calibrate_multi_seed`,
plumbed through the bench end-to-end.

**Phase A scout** (`.scratch/era-13-5/phase-a-scout.md`) measured the
expected effect on six target fixtures (5 cluster-rare residuals + Phase 4a
ER target). Result: **3/6 unblocked under asym** (all 3 faker-js cluster-rare
residuals); threshold drop is exactly 5.0 = `cluster_bonus` on all three
corpora (perfectly uniform). The other three failed for two reasons not
addressable by Phase A:
- **fastapi `validation_2`, `exception_handling_4`**: parse error on bare
  hunk → `_has_root_error=True` → call_receiver returns 0 before any bonus
  is computed. asymmetric calibration cannot help.
- **hono `hono_validation_2`**: raw BPE 2.31 is far below even the asym
  threshold (4.35) — single attested callee `c.json`.

**Phase A scoped bench** (faker-js + fastapi, asym-cal flag enabled):
- faker-js: 16/17 (94.1%), FP 2.0% — **G2 cleared, +3 catches**
- fastapi: 30/32 (93.8%), FP 13.4% — **G2 blown 5×**

The faker-js win was real; the fastapi result confirmed PRD § Risk R1 firing
exactly as documented. Across the other 4 corpora measured later, asym was
universally unsafe (5/6 corpora FP-flooded). The mechanism is corpus-
dependent, which led directly to the auto-detect work.

### Phase B — Negative-shape primitive (commit `58c3a58`)

**Verdict: ships as registered primitive, default off.**

`typical_call_density` (math option iii: density of cluster's top-10 attested
callees in the hunk) was scout-validated at z=+10.17 on `synthetic_formula_1`
(faker primary target), with the asymmetric-by-construction premise validated
on cluster files (0/10 fired the math). Implemented per
`engine/argot/scoring/scorers/typical_call_density.py` and registered in the
shape-primitive registry alongside the existing four primitives. 11 unit
tests cover the abstain conditions and language-agnostic behavior.

**Phase B scoped bench** (faker + ink + hono, asym-cal + primitive enabled):
all three corpora FP-flooded (7-17% real-PR FPs). The
asymmetric-by-construction premise validated on cluster files (model_a
sources) but NOT on real-PR control hunks — a methodological blind spot in
the Phase B scout. The primitive remains registered for era-14 work to
build on; default-off so production behaviour is unchanged.

### Phase C — Host-context AST for `call_scope_fraction` (DROPPED)

**Verdict: dropped per PRD's pre-registered conditional rule.**

Phase C scout measured CSF tail-z on `runtime_fetch_1` (faker-js) reading
the spliced-into-host AST instead of the bare hunk. Era-13 had reported
−1.58σ on bare hunk; expectation was a sign flip with usable positive z
under host-context.

Result: **no sign flip.** Bare-hunk z = −1.34σ (matches era-13 within drift);
host-AST z = −1.16σ. Both magnitudes below 1σ ramp — primitive wouldn't fire
under either input shape. The hunk's own `function_declaration` wraps the
calls regardless of host position.

Phase C dropped per the PRD's explicit conditional gate (§ Phase C decision
rule). The era-13 design-contract reopening is not load-bearing without a
positive scout.

**Bonus finding (era-14 backlog item):** the existing CSF primitive uses
`_FUNCTION_BOUNDARY = "function_definition"` for both Python AND TypeScript,
but TypeScript uses `function_declaration` not `function_definition`. CSF
universally returns fraction=1.0 on TS files → std=0 → z=0 → primitive
ALWAYS ABSTAINS on TypeScript corpora. Doesn't affect era-13.5 ship (CSF
disabled by default), but is real research debt.

### Auto-detect mechanism (post-PRD, commit `58c3a58`)

**Verdict: the era's actual headline.**

After the three pre-registered phases closed with the conclusion "asym is
informative on faker-js, FP-floods elsewhere," the question shifted from
"can we make asym safe everywhere?" to "can we predict per-corpus whether
asym is safe?".

A series of experiments (`.scratch/era-13-5/`) ruled out four diagnostic
proxies before validating the right signal:

| Experiment | Signal | Result |
|---|---|---|
| Cal-as-proxy (random source-file sampler) | predicted FP from cal score distribution | broken — off by 4-25× from actual real-PR FP |
| Single-PR diff-cal | replace random sampler with extracted diff hunks | tightened proxy on faker-js (3.8% vs actual 2.0%); other corpora still mispredict |
| Multi-PR diff-cal (5 PRs, 500 hunks) | broader diff sample | thresholds shift unpredictably per corpus; drops faker-js catches |
| Concentration-based rule (n_rare/n_total per hunk) | fixture vs control concentration | only faker-js fixtures show high concentration; doesn't generalize |
| **Per-hunk fire-rate on diff hunks** | **fraction of diff hunks where cluster_rare fires at least once** | **clean separation: faker-js 2.2% vs others 10-22% (5× margin)** |

**The validated signal**: at fit time, probe `cluster_rare` on N=1000 diff
hunks (loaded from `extract`'s `dataset.jsonl`). Count distinct hunks
firing the rule (per-hunk fire rate). If < 5% → enable Phase A asymmetric
calibration. Otherwise → disable cluster_rare entirely (= baseline).

**Implementation** (commit `58c3a58`):
- `engine/argot/scoring/scorers/call_receiver.py`: added
  `rare_branch_hunks_fired` and `hunks_scored` counters (per-hunk
  granularity, not callee-level — robust to "many fires per hunk" vs
  "few fires per hunk" which varies per corpus).
- `engine/argot/scoring/scorers/sequential_import_bpe.py`: exposed
  counters as properties.
- `benchmarks/src/argot_bench/score.py`: `auto_select_asym_cal`,
  `asym_fire_rate_threshold`, `auto_detect_probe_dataset` parameters
  on `build_scorer`. The probe loads from extracted `dataset.jsonl`
  when available (the bench provides it via `auto_detect_probe_dataset`);
  falls back to random-source sampling otherwise.
- `benchmarks/src/argot_bench/{run.py, cli.py}`: plumbed flags through
  `RunConfig` and `--auto-select-asym-cal` / `--asym-fire-rate-threshold`
  CLI arguments.
- Default `--auto-select-asym-cal=False`: opt-in via the CLI flag, so the
  era-11 / era-13 production behavior is preserved as default. Set the
  flag plus `--call-receiver-cluster-rare-threshold=2` to activate.

**Why the signal works.** faker-js is structurally unique: TS library with
uniform clusters (faker providers all use cluster-typical callees). PR hunks
in faker-js use those typical callees → rare-rule almost never fires on
controls. Catalog breaks introduce foreign callees (`Math.random`, `fetch`)
→ rule fires hard. Clean signal/noise separation.

The 5 other corpora are heterogeneous (frameworks, polyglot, doc-rich). PR
hunks legitimately use Zipf-tail rare callees in normal code → rule fires
often → enabling it would FP-flood. Auto-detect correctly disables it for
these.

## Era-13.5 ceiling reached

The PRD pre-registered a 9/10-catchable architectural ceiling
(114/115 = 99.1%). Era-13.5 catches +3 of those, leaving 7/10 still uncaught.
The 6 residuals fall into two structural buckets that **era-13.5's mechanisms
cannot reach**:

1. **Parse-error blocked** (fastapi `validation_2`, `exception_handling_4`):
   `_has_root_error=True` on bare hunk → call_receiver returns 0. Any
   call-receiver-derived bonus mechanism is structurally dead. Needs host-
   AST scoring at the call-receiver level (era-12 routing fix was about
   cluster lookup; this is about parsing the hunk itself).

2. **Structural anomaly outside callee framing** (faker `synthetic_formula_1`,
   ink `ink_dom_access_2`, hono `hono_middleware_3`, hono `hono_validation_2`,
   faker-js `error_flip_2`): hunks with 0-2 callees that are themselves
   unremarkable; the anomaly is in the absence of cluster-typical patterns or
   in control-flow shape. `typical_call_density` shows promise on a subset
   (Phase B scout caught `synthetic_formula_1` standalone) but FP-floods at
   the bench level.

The era-14 backlog (see "What's next" in `docs/research/README.md`) targets
these two buckets with new mechanisms.

## Architectural decisions ratified by this era

1. **Asymmetric calibration is mathematically sound** when the cal hunk
   distribution is structurally separable from the real-PR control
   distribution on the cluster_rare signal. The G7 contract
   (`docs/agents/calibration-contract.md`) makes this binding so future
   contributors don't rip it out as a "cal scores one way, scores another"
   smell.

2. **Per-corpus auto-detect is the right abstraction** when a mechanism
   helps some corpora but hurts others. Rather than gate the mechanism
   behind a per-corpus config (manual decision per repo), probe the
   data-driven signal at fit time and decide automatically.

3. **Diff hunks as a calibration distribution** are structurally different
   from random source-file windows. The current production calibrator uses
   random source windows; the auto-detect probe uses diff hunks
   specifically (loaded from extract's output). A future era-14 question:
   should the calibrator itself switch to diff hunks?

## Files shipped

Production code (commit `58c3a58`):
- `engine/argot/scoring/calibration/__init__.py` — `apply_optional_contributions_to_cal` flag
- `engine/argot/scoring/scorers/call_receiver.py` — per-hunk fire counters
- `engine/argot/scoring/scorers/sequential_import_bpe.py` — counters exposed
- `engine/argot/scoring/scorers/typical_call_density.py` — Phase B primitive
- `engine/argot/scoring/scorers/shape_primitive_registrations.py` — primitive registered
- `engine/argot/tests/test_calibration.py` — 3 tests for asym flag
- `engine/argot/tests/test_typical_call_density.py` — 11 tests for primitive
- `benchmarks/src/argot_bench/score.py` — `auto_select_asym_cal` mechanism
- `benchmarks/src/argot_bench/{run.py, cli.py}` — flag plumbing
- `docs/agents/calibration-contract.md` — G7 contract doc

`just verify` and `just verify-bench` clean (303 + 109 tests, 0 errors).

## Reproduce

```bash
uv run --directory benchmarks argot-bench \
  --auto-select-asym-cal \
  --call-receiver-cluster-rare-threshold=2
```

Expected output: 108/115 = 93.9% recall, all per-corpus FP ≤ 2.0%, threshold
CV 0.0% under K=7 multi-seed.

Canonical bench results: `benchmarks/results/baseline/latest/` (committed).
