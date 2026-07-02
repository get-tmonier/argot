# G7 Calibration Contract

**Status:** binding (era-13.5 gate G7)
**Code:** `crates/argot-core/src/scoring/calibration.rs` — `run_calibrate` (multi-seed median threshold)

---

## The problem: symmetric firing cancels recall

During era-13 Phase 10 development, every new optional contribution tested
(cluster-rare-threshold rule, Phase-4 ShapePrimitive penalties) was measured
against the calibrated threshold and found to add ~zero recall.  The cause is
structural:

> Any additive contribution that fires **symmetrically** on calibration hunks
> and fixture hunks inflates the per-corpus threshold by the same magnitude it
> adds to fixture scores.  The net catch impact is ~zero.

Empirical evidence: the era-13 Phase A scout (`docs/research/evidence/era13-final.md`
§ cancellation) confirmed that with `cluster_rare_threshold=2`, all three corpora
(faker-js, fastapi, hono) had exactly 200–280 rare-branch fires per calibration
seed.  The threshold rose by exactly `cluster_bonus = 5.0` — matching the
observed scout `t_sym − t_asym = 5.0000` on every corpus.

---

## Why suppressing on the cal path is mathematically sound

The cancellation argument assumes both paths use *the same* scorer.  The fix is
to break that symmetry **at the calibration boundary**, not inside the scorer.

### Era-11 cluster_bonus is already asymmetric by construction

Calibration hunks are sampled from `model_a_files` — files whose callee bags are
typical for their cluster.  By construction, these files' callees are **within**
their cluster's attested set, so the cluster-absent-callee branch of
`weighted_contribution_for_file` does not fire on calibration hunks.  Era-11's
`cluster_bonus` is asymmetric without any special plumbing.

### New optional contributions are not asymmetric by construction

The cluster-rare rule fires when a callee is attested in ≤ N cluster files — a
Zipf-distribution property shared by typical code and anomalous code alike.
Calibration hunks from `model_a_files` call rare-attested functions too (e.g.
build utilities, test helpers that appear in one file).  Shape primitives based
on AST patterns (exception handling ratios, call scope fractions) also fire
on calibration hunks unless the corpus is carefully curated.

---

## The contract

`calibrate_multi_seed` accepts `apply_optional_contributions_to_cal: bool = False`.

**When `False` (default):** calibration scorers are built with
`cluster_rare_threshold=0` and `shape_primitives=[]`.  The threshold reflects
only what typical code scores under the base scorer plus the era-11 cluster_bonus
(which remains symmetric — i.e., cluster-absent callees, which don't fire on cal
by construction).

**When `True` (symmetric mode):** calibration scorers use the full passed
parameters.  Use only for explicit comparison; this reproduces the era-13 status
quo where optional contributions cancel.

### What is identical on both paths

| Component | Cal path | Fixture path |
|:---|:---:|:---:|
| Base BPE scorer | ✓ | ✓ |
| Era-11 cluster_bonus (cluster-absent callees) | ✓ | ✓ |

### What differs (optional contributions)

| Component | Cal path (flag=False) | Fixture path |
|:---|:---:|:---:|
| cluster_rare_threshold rule | suppressed (0) | full value |
| ShapePrimitive penalties | suppressed ([]) | full list |

The suppression is applied **only to the calibration scorer** — the
`SequentialImportBpeScorer` used for fixture and real-PR scoring always receives
the full parameters from `build_scorer`.

---

## Invariant

```
T(flag=False, rare_threshold=R, primitives=P)
  == T(flag=True, rare_threshold=0, primitives=[])
```

With `flag=False`, the calibration threshold is bit-identical to a run where the
caller never passed any optional contributions at all.  This means the default
(`flag=False`) is a strict no-op under the era-11 production config (where
`rare_threshold=0` and `primitives=[]` already).  New configs that set
`rare_threshold > 0` or enable primitives must pass `flag=False` to get the
asymmetric threshold; the flag default ensures they do so without explicit opt-in.

---

## Failure mode if the contract is violated

If a future maintainer passes optional contributions to the calibration scorer
without understanding this contract, the threshold will rise by the contribution's
calibration-fire magnitude.  The contribution will appear to add recall (fixtures
score higher), but the threshold will rise by the same amount, leaving net catch
impact at ~zero.  The era-13 research (`docs/research/evidence/era13-final.md`
§ Phase 2 sweep and § Phase 10) documents exactly this failure mode across 12
sweep cells and 4 Phase 4 primitive compositions.

The `[rare-counter]` stderr line from `calibrate_multi_seed` includes
`asym_cal=True` when the flag is False and a non-zero `rare_threshold` was
passed, making the asymmetric mode observable end-to-end.

---

## References

- `docs/research/evidence/era13-final.md` — era-13 final memo with § cancellation
- `crates/argot-core/src/scoring/calibration.rs` — implementation
- `crates/argot-core/tests/calibration_smoke.rs` — calibration tests

---

## Era-14 extensions

### Rarity weighting (phase A)

`RarityWeighting` (crates/argot-core/src/scoring/call_receiver.rs) scales the
cluster-branch bonuses (`cluster_absent`, `cluster_rare`) by a weight derived
from the callee's corpus-global document frequency (number of corpus files
whose callee bag contains it). Formula shapes: `LinearDf` (df/N), `GatedDf`
(1 if df ≥ M else 0), `LogDf` (ln(1+df)/ln(1+N)); `Off` is the era-13.5
behaviour and the production default.

The weighting extends the asymmetry-by-construction argument from *firing
rate* to *magnitude*: it applies identically on the calibration and scoring
paths (probe, cal, and check call-receivers are all built with the same
weighting), so any FP reduction comes from the weight's correlation with the
callee population, not from a path asymmetry. Scout evidence (era 14 phase A
scout, recorded in `docs/research/evidence/era14-final.md`) shows the df axis
does NOT separate break callees from FP callees on the pain corpora —
foreign-paradigm break callees are rare in-corpus by construction — so the
weighting ships as substrate, default `Off`, unless a scoped bench shows a
formula that cuts FP without recall regression.

### Diff-hunk calibration source (phase B)

`--calibration-source diff` (bench) calibrates against real diff hunks from
the extract dataset instead of random source-file ranges. Scope filters stay
in lock-step with control scoring: excluded paths are dropped, hunks are
language-matched, and there is deliberately NO minimum-hunk-size filter — the
threshold must reflect exactly the population the checker scores, including
tiny hunks. File content is read at the extraction commit (`git show`), never
the current checkout, so line bounds cannot go stale. The random source
remains the default until a cumulative bench validates diff-cal against G1/G2.

### Parse-error host fallback (phase D)

When a bare hunk's parse has root-level ERROR nodes, callee extraction falls
back to the hunk's line region within its host file's AST (real-PR path:
`file_source` + hunk bounds; catalog path: synthesized hunk-in-host content).
Calibration candidates carry their own file region, so the threshold side
sees the same fallback — cal/score symmetry is preserved. Hunks that parse
cleanly never consult host context (G4.d, unit-tested in call_receiver.rs).

---

## Era-15 extensions

### The model artifact carries the calibration's world

`scorer-config.json` v3 persists the fit-time model (BPE token counts,
callee attestation, cluster partition, convention frequencies + bars).
Check scores against this snapshot, never the live tree — the threshold and
the check path now see one score distribution by construction (issue #79:
live-tree rebuilds let new code attest itself, silently disabling the
unattested-callee branches on exactly the code check judges).

### Parse-error host fallback is ON at check time

The calibration side always applied the fallback; check ran with it off
(era-14 gating, measured under a forced cluster-rare rule). Since git picks
hunk boundaries, bare-fragment parse errors are the *norm* for check-time
hunks — fallback-off made the call-receiver contribute 0 on most real
staged hunks while the threshold included fallback-carrying cal scores.
`run_calibrate` emits `call_receiver_parse_error_host_fallback: true` and
check honours it; symmetry restored. The bench reproduces the era-14
configuration with `--no-parse-error-fallback`.

### Cluster branches require fitted membership

Cluster-conditional branches (`cluster_absent`, `cluster_rare`) fire only
for files present in the fit-time `file_to_cluster` map. Jaccard-guessing a
cluster for an unknown file hands its own staples wrong-cluster bonuses (a
React file routed into an Effect-heavy cluster was the dominant FP driver
on real new-feature commits). Calibration candidates are corpus files and
path-route after repo-dir canonicalization, so the cal side is unaffected;
`nearest_cluster_for_source` survives for evidence display only.

### Convention-rarity stage (asymmetric by calibrated bar)

The convention stage (AST node-kind surprisal + identifier-shape surprisal,
`scoring/conventions.rs`) is an additive bonus like the cluster rules and
follows the same contract discipline: it never feeds
`multi_seed_thresholds`. Instead of a fire-rate probe, its asymmetry is
enforced by construction — the firing bars are the **max** feature value
over the same multi-seed calibration sample the threshold uses, so the
stage is silent on the calibration population by definition and the
threshold is bit-identical with the stage enabled or disabled.

### Neighbourhood attestation on the scoring side only

The unattested-callee branches skip callees the change itself binds
(callable definitions — file- and changeset-wide, import bindings, local
value bindings for bare calls) and dotted callees whose method segment is
corpus-known. The calibration side scores without these exclusions
(candidates are corpus files whose callees are attested anyway); the
omission leaves the threshold marginally conservative, which is the safe
direction under the cancellation argument.
