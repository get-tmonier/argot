# G7 Calibration Contract

**Status:** binding (era-13.5 gate G7)
**Code:** `engine/argot/scoring/calibration/__init__.py` — `calibrate_multi_seed`

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
- `engine/argot/scoring/calibration/__init__.py` — implementation
- `engine/argot/tests/test_calibration.py` — unit tests (a/b/c)
