# Era 14 — Final Evidence

Era 14's brief (issue #54, PRD at `.scratch/era-14/PRD.md`): halve false
positives so all six corpora sit ≤ 1.0% FP, close the residual recall gap
toward the 114/115 architectural ceiling, and clear the era-13.5 backlog —
comprehensively, so an era 15 isn't needed. Six phases (A–F) ran to their
pre-registered decision points on the new Rust bench harness
(`crates/argot-bench`), which first reproduced the era-13.5 baseline exactly
(108/115, identical uncaught set, per-corpus FP within sampling noise).

## Headline

**Ship the era-13.5 scoring defaults unchanged; ship phases E and F; ship
phases A–D as engine substrate, default-off.** G1 (≤ 1.0% FP everywhere) is
NOT cleared: all four candidate mechanisms either miss their targets or
violate the recall floor / FP ceiling, each with a specific, now-documented
structural reason. The era's durable wins are production wins, not scorer
wins: the auto-detect calibrator finally reaches `argot calibrate` (the Rust
port had silently pinned production at the pre-13.5 baseline), and the
benchmark harness exists again.

| Gate | Target | Result | Verdict |
|:---|:---|:---|:---|
| G1 FP ceiling | all 6 ≤ 1.0% | faker 1.9–2.1%, faker-js 1.7%, rich 1.23% unchanged | **not cleared** |
| G2 recall floor | ≥ 108/115 | 108/115 (ship config) | cleared |
| G3 recall stretch | ≥ 114/115 | 108/115 (110/115 reachable only with phase D's FP cost) | not cleared |
| G4 no regression | 0 of 108 | 0 regressions in ship config | cleared |
| G6 no domain knowledge | — | all weights from corpus statistics | cleared |
| G7 contract | lands with code | `docs/agents/calibration-contract.md` era-14 sections | cleared |
| G8 auto-detect debt | delete or productionise | **productionised** into `argot calibrate` | cleared |

## Phase A — rarity weighting (REFUTED)

Mechanism: scale the cluster-branch bonuses by the callee's corpus-global
document frequency (`--rarity-weighting linear-df | gated-df:M | log-df`).
PRD erratum recorded first: the PRD's `idf = log(N/n)` formula contradicts its
own intent ("globally-rare → weight ≈ 0"); the implementation follows the
intent (weight increases with df).

Scout (all six corpora, ~1000 real-PR hunks + all fixtures, per-callee df on
every cluster-branch event): **the df axis does not separate break callees
from FP callees.** Foreign-paradigm break callees are rare in-corpus *by
construction* — `fetch` appears in 1 of 1002 faker-js files; `Router`, `cors`,
`fake.name` all at df ≤ 2 — the same band as 65–85% of cal-side FP events
(faker-js fixtures: 18/21 cluster events at df ≤ 3; faker: 8/9; hono: 19/20).

Scoped benches (faker + faker-js) confirmed:

| Formula | faker FP | faker-js recall | faker-js FP |
|:---|---:|---:|---:|
| off (baseline) | 2.10% | 16/17 | 1.70% |
| linear-df | 1.54% | **9/17** | 0.83% |
| gated-df:2 | 2.09% | **12/17** | 2.46% |
| log-df | 1.89% | **10/17** | 1.42% |

Every formula buys FP by destroying the cluster-branch catches (G4.a
violated). Thresholds are unchanged across formulas — the weighting removes
fixture-side bonuses without moving the calibration max. Ships default `Off`.

## Phase B — diff-hunk calibration (REFUTED at default thresholding)

Mechanism: calibrate against real diff hunks from `dataset.jsonl`
(`--calibration-source diff`), scope-locked to control scoring. Result:
thresholds explode (8.8–11.9 vs 3.8–5.3) because real diff fragments are
high-BPE-surprise, and the max/K-seed-median threshold tracks the sample
maximum. FP → ~0 everywhere but recall collapses to 63/115 (54.8%). The
era-13.5 night-log instability was directionally right. Ships as substrate,
default `random`. (Pairing diff-cal with sub-max percentiles is a different
experiment; era-13 recorded percentile thresholding as monotonically worse,
so it was not retried per the do-not-retry table.)

## Phase C — negative-shape primitive (REFUTED, all three maths)

Three pre-registered maths for "absence of cluster-typical patterns":
(i) `cluster_staple_deficit` (top-10 deficit), (ii)
`callee_distribution_under_coverage` (one-sided smoothed KL), (iii)
`typical_call_density` (era-13.5 re-attempt under the new substrate). Scoped
benches on faker + ink + hono:

| Primitive | targets caught | faker FP delta |
|:---|:---|---:|
| typical_call_density | 0 of 3 | +0.44pp |
| cluster_staple_deficit | 0 of 3 | +0.83pp |
| callee_distribution_under_coverage | 0 of 3 | +0.22pp |

All three fire on real-PR controls and none reaches `synthetic_formula_1`,
`ink_dom_access_2`, or `hono_middleware_3`. The rejection rule (fire on
controls without target catches) applies to every option: wrong shape.
Registered, default-off.

## Phase D — parse-error host fallback (WORKS for recall, FP-couples; gated)

Mechanism: when a bare hunk's parse has root-level ERROR nodes, extract
callees from the hunk's region within the host AST (real-PR: file_source +
bounds; catalog: synthesized hunk-in-host). Narrow, pre-registered reopening
of the era-13 host-AST ban (callee extraction only). G4.d invariant
unit-tested: clean-parsing hunks never consult host context.

Layer-D bench: **fastapi 32/32 — first 100% corpus; both parse-error
residuals (`validation_2`, `exception_handling_4`) caught** at unchanged
fastapi FP (0.57%). But on corpora where the cluster-rare rule is active the
same fallback lets real-PR parse-error fragments collect rare bonuses the
threshold never saw: faker-js 1.70% → 4.86% FP (past even the era-11 2.5%
ceiling). Disabling the rare rule there instead would drop the era-13.5
catches (faker-js → 13/17; total 107 < 108 = G2 violation). Diff-cal cannot
absorb it (phase B). Ships gated: `call_receiver_parse_error_host_fallback`
(engine) / `--enable-parse-error-fallback` (bench), default off. The +2
fastapi catches are one threshold-mechanism away for era 15.

## Phase E — auto-detect productionised (SHIPPED)

The final Python production calibrator shipped era-13.5 (`rare=2` +
per-corpus auto-detect); the Rust port hardcoded `rare=0`, silently pinning
production at the pre-13.5 baseline. `argot calibrate` now probes the rare
rule's fire rate per language and emits the resolved threshold into
`scorer-config.json` (`--call-receiver-cluster-rare-threshold`,
`--no-auto-select-asym-cal`, `--asym-fire-rate-threshold`). Deletion was
rejected because phase A's refutation shows rarity weighting does NOT
obviate per-corpus gating.

## Phase F — CSF TypeScript boundary (SHIPPED)

`call_scope_fraction` used tree-sitter-python's `function_definition` as the
scope boundary for both grammars, so every TypeScript call looked
module-scope (fraction constantly 1.0 → std 0 → permanent abstain on TS).
Boundary is now per-grammar; the two goldens that locked the quirk were
updated as an intentional behaviour change.

## Ship configuration (cumulative bench)

Defaults unchanged from era-13.5: `n_cal=100`, K=7 seeds, α=2.0, cap=5,
clusters=8, bonus=5.0, rare=2 + auto-detect, random calibration, no
primitives, no rarity weighting, no parse-error fallback.

| Corpus | Type | Recall | FP rate | AUC | Threshold |
|:---|:---|---:|---:|---:|---:|
| fastapi | library | 30/32 (93.8%) | 0.53% | 0.995 | 5.26 |
| rich | library | 16/16 (100%) | 1.23% | 0.996 | 3.84 |
| faker | library | 15/16 (93.8%) | 1.92% | 0.954 | 5.07 |
| hono | library | 15/17 (88.2%) | 0.51% | 0.833 | 4.27 |
| ink | library | 16/17 (94.1%) | 0.39% | 0.991 | 4.99 |
| faker-js | library | 16/17 (94.1%) | 1.70% | 0.948 | 4.86 |
| saleor | application | 12/14 (85.7%) | 0.24% | 0.993 | 5.44 |
| wagtail | application | 14/14 (100%) | 0.34% | 0.999 | 4.67 |
| excalidraw | application | 9/14 (64.3%) | 0.43% | 0.957 | 5.76 |
| outline | application | 10/14 (71.4%) | 0.46% | 0.879 | 5.00 |

Libraries: **108/115 (93.9%)**, identical uncaught set to era-13.5 (zero
regressions, G2/G4 confirmed). Applications: **45/56 (80.4%)**, FP ≤ 0.5%
on every application corpus. Full report:
`benchmarks/results/baseline/latest/report.md`.

## Era-15 seeds

- A threshold mechanism that survives diff-hunk calibration (sub-max
  aggregation with a recall guard) would unlock BOTH phase B's honesty and
  phase D's +2 catches — the two refutations share the same root (max-of-
  sample thresholding).
- The FP pain (faker locale tail, faker-js Zipf tail) is invariant to every
  frequency axis tried across eras 13–14; per-callee frequency is exhausted
  as a family.
- Threshold CV across inner seeds (reported per corpus by the new harness) is
  6–14% on several corpora — well above the historical outer-seed CV ≈ 0
  metric. The G5 "CV ≤ 3%" gate needs a redefinition before era 15 gates on it.
