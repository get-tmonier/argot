# Era 9 — Alpha Sweep: call_receiver_alpha tuning

> **TL;DR.** Raise `call_receiver_alpha` from 1.0 to 2.0. Faker-js gains 2
> fixtures (http_sink_1, http_sink_3), hono gains 1 (routing_3), and ink gains
> 1 (dom_access_1). Avg recall improves from 80.57% → 84.4% (+3.8 pp). All 6
> gates pass. Four fixture labels moved uncaught→hard.

## Hypothesis

Era-8 shipped with α=1.0. Examination of the remaining faker-js misses showed
that `http_sink_1` (axios, BPE 3.281) and `http_sink_3` (navigator.sendBeacon,
BPE 2.841) each have exactly one unattested callee. At α=1.0 their adjusted
scores are 4.281 and 3.841 — both below the faker-js threshold of 4.773.
Raising α should push them over.

Formula: `adjusted = raw_bpe + α · min(n_unattested, C)` where C=5 (cap).

The predicted crossing point is α≥1.5 for http_sink_1 (needs adj ≥ 4.773; raw
3.281 + 1.5·1 = 4.781 ✓) and α≥2 for http_sink_3 (raw 2.841 + 2·1 = 4.841 ✓).

Pre-registered primary: α=3.0. Fallback: α=2.0 (if Gate 3 FP fails).
Recovery: α=5.0 (if Gate 5 faker-js recall fails at α=3.0).

## Results — alpha=3.0 (primary, 20260424T154424Z)

| Corpus | Era-8 recall | α=3.0 recall | Δ | FP |
|:---|---:|---:|---:|---:|
| fastapi | 91.7% | 91.7% | 0 | 0.8% |
| rich | 95.0% | 95.0% | 0 | 0.8% |
| faker | 95.0% | 95.0% | 0 | 1.6% |
| hono | 71.7% | 88.2% | +16.5pp | 1.2% |
| ink | 86.7% | 100.0% | +13.3pp | 0.4% |
| faker-js | 43.3% | 52.9% | +9.6pp | 1.1% |

**Gate 3 failed:** faker FP 1.6% > 1.5% ceiling. Triggered fallback to α=2.0.

### Surprise gains at α=3.0

Hono gained 3 unexpected fixtures (framework_swap_1, middleware_2, routing_3)
and ink gained 2 (dom_access_1, dom_access_2). These fixtures have multiple
unattested callees whose combined penalty at α=3.0 clears the threshold. The
gains are real, but the FP cost at α=3.0 is too high to ship.

## Results — alpha=2.0 (fallback, 20260424T163221Z)

| Corpus | Era-8 recall | α=2.0 recall | Δ | FP |
|:---|---:|---:|---:|---:|
| fastapi | 91.7% | 91.7% | 0 | 0.8% |
| rich | 95.0% | 95.0% | 0 | 0.8% |
| faker | 95.0% | 95.0% | 0 | 1.2% |
| hono | 71.7% | 78.3% | +6.6pp | 0.5% |
| ink | 86.7% | 93.3% | +6.6pp | 0.4% |
| faker-js | 43.3% | 53.3% | +10.0pp | 1.0% |

Avg recall: 80.57% → 84.4% (+3.8 pp).

## Gate verdicts (alpha=2.0)

| # | Gate | Threshold | Observed | Verdict |
|---|---|---|---|---|
| 1 | Avg recall ≥ 83.5% | 83.5% | 84.4% | ✓ |
| 2 | No corpus regresses > 1 fixture | — | all improved | ✓ |
| 3 | All corpora FP ≤ 1.5% | 1.5% | max 1.2% (faker) | ✓ |
| 4 | 0 category regressions from 100% | 0 | 0 | ✓ |
| 5 | faker-js ≥ 9/17 | 9/17 | 9/17 (53.3%) | ✓ |
| 6 | hono ≥ 12/17 | 12/17 | 13/17 (78.3%) | ✓ |

All 6 gates pass. α=2.0 ships.

## Fixtures moved

| Fixture | Old label | New label | Mechanism |
|---|---|---|---|
| faker_js_http_sink_1 | uncaught | hard | axios.post is unattested; adj 3.281+2=5.281 > 4.773 |
| faker_js_http_sink_3 | uncaught | hard | navigator.sendBeacon is unattested; adj 2.841+2=4.841 > 4.773 |
| hono_routing_3 | uncaught | hard | app.all unattested + ≥1 additional unattested callee; adj 0.819+4=4.819 > 4.277 |
| ink_dom_access_1 | uncaught | hard | document.getElementById + window.addEventListener unattested; adj 2.105+4=6.105 > 4.826 |

## Why http_sink_2 remains uncaught

`faker_js_http_sink_2` uses `fetch` (BPE 3.767, 0 unattested callees). `fetch`
is attested in the faker-js corpus — it appears in legitimate PR hunks. Alpha
tuning cannot help; the fixture is a structural gap.

## Surprising observation

`hono_routing_3` was annotated in era-8 as "not a complex-chain issue" because
`app.all` is identifier-rooted and attested in the Hono corpus. Yet at α=2.0 it
flips to caught. The reason: the fixture also calls `(req, res) => ...` with
`res.send` — `res.send` is an Express receiver that is absent from the Hono
corpus. With two unattested callees, the adjusted score is 0.819 + 2·2 = 4.819
which clears the hono threshold of 4.277.

Era-8's era-8 narrative said "catching hono_routing_3 would require a structural
pattern scorer." This was wrong — a correctly tuned α was enough.

## What remains uncaught

| Fixture | Score | Why |
|---|---|---|
| faker_js_http_sink_2 | 3.767 | fetch is attested; 0 unattested callees |
| faker_js_foreign_rng_1/3 | 0.520 | Math.random is attested; BPE alone too low |
| hono_framework_swap_1 | 1.484 | Express Router() idiom; BPE very low, complex-chain |
| hono_middleware_2/3 | 0.110 / −1.736 | structural breaks; no foreign callee |
| hono_validation_2 | 2.231 | typeof guards; no foreign callee, BPE too low |
| ink_dom_access_2 | 4.215 | window.location.href; score below ink threshold |

## Decision trail

1. α=3.0 primary bench completes: faker FP 1.6% → Gate 3 fail.
2. Per pre-registered decision tree: trigger α=2.0 fallback.
3. α=2.0 bench completes: all 6 gates pass.
4. Update default in `engine/argot/scoring/scorers/sequential_import_bpe.py` and
   `benchmarks/src/argot_bench/cli.py`. Fix stale test in `benchmarks/tests/test_cli.py`.
5. Relabel 4 fixtures across 3 manifests.
6. Promote run `20260424T163221Z` as canonical baseline.
