# Era-13.5 asymmetric calibration is now wired into production

> **TL;DR.** PR #38 added per-corpus auto-detect of the cluster-rare
> attestation rule and shipped a +16.6pp recall win on faker-js — but
> only inside the bench harness. Production `argot-calibrate` hardcoded
> the rule off, so end users running `argot fit` got the pre-13.5
> baseline. This branch ports the auto-detect probe + asym-calibration
> wiring into `argot-calibrate` and makes both the default. Faker-js
> recall on `just bench` is back at 93.3%; the other five corpora are
> unchanged.

## Context

The cluster-rare rule (era-13 phase 10): repo files cluster by callee
overlap (KMeans on callee bags, K=8 default). Inside each cluster, a
callee that's globally attested but appears in ≤ N files gets treated
as cluster-absent and incurs `cluster_bonus`. The intent: catch a hunk
that calls `db.dangerouslyExec` when the cluster overwhelmingly uses
`db.query` / `db.commit`, even though `dangerouslyExec` is *technically*
attested somewhere.

The catch: the rule fires symmetrically on calibration hunks too. Cal-
side firing inflates the threshold, cancelling the recall contribution
on the fixture path. Two ways out:

1. **Symmetric (default pre-13.5).** Rule on for both cal and scoring,
   threshold absorbs the contribution, no recall win.
2. **Asymmetric (era-13.5).** Rule off on cal, on for scoring. The
   contribution lands only on the fixture path, recall win materialises.

But asym is corpus-dependent. On a codebase where the rule fires often
on ordinary code (real PRs trip it as much as catalog breaks), enabling
asym would FP-flood. Era-13.5's solution: probe per-corpus. Build a
probe scorer with the rule enabled, measure cal-side per-hunk fire
rate. If `< 5%` the rule is discriminative → keep (asym). Otherwise
disable (symmetric, baseline behaviour).

The bench wired all this into `benchmarks/src/argot_bench/score.py`,
exposed `--auto-select-asym-cal` and `--call-receiver-cluster-rare-
threshold` CLI flags, and produced the era-13.5 baseline numbers
(`auto-detect-final/`). But the wiring stopped at the bench. Production
`argot-calibrate.main()` hardcoded the rare-threshold to 0 and never
ran the probe.

The miss surfaced when the bench wrapper got fixed in this branch
(`5c2833e` — `score.py` was still subscripting `ScoredHunk` after it
became a dataclass in `f834dc6`). Re-running `just bench` on the
default flags showed faker-js at 76.7%, the era-13.5 numbers were
93.3%. Bisect confirmed the regression wasn't in the engine — the
era-13.5 win was a bench-only feature that production never inherited.

## Decision

1. Add four CLI args to `argot-calibrate`:
   - `--call-receiver-cluster-rare-threshold` (default `2`, era-13.5
     setting)
   - `--call-receiver-cluster-size-min` (default `0`)
   - `--auto-select-asym-cal` (`BooleanOptionalAction`, default on —
     pass `--no-auto-select-asym-cal` to opt out)
   - `--asym-fire-rate-threshold` (default `0.05`)
2. Port the probe into `argot-calibrate`: build a one-off scorer with
   the rule enabled, measure `rare_branch_hunks_fired / hunks_scored`,
   keep or disable accordingly. Mirrors the bench logic exactly.
3. Thread the resolved `cluster_rare_threshold` and `cluster_size_min`
   through `calibrate_multi_seed` and the final scorer construction.
4. Persist resolved values to `scorer-config.json` under new keys; load
   them in `check.py:_load_phase14_scorer`.
5. Mirror the new defaults in `benchmarks/src/argot_bench/cli.py` so
   `just bench` reflects what end users will actually see, not opt-in
   bench territory.

## Why default-on auto-detect

Auto-detect is the safe default by construction:

- On corpora where the rule helps (faker-js: 1.3% cal fire rate), it
  enables and adds catches.
- On corpora where the rule would FP-flood (≥ 5% cal fire rate), it
  silently disables and falls back to baseline behaviour — which is
  exactly what the pre-13.5 default was.

There's no corpus where auto-detect is *worse than* hardcoded-off. The
cost is one extra scorer build per `argot fit` (~30s) for a probe;
acceptable for a fit-time operation.

## Verification

Bench numbers before/after this change (six-corpus run, default flags):

| Corpus   | Recall before | Recall after | AUC before → after |
|----------|---------------|--------------|--------------------|
| fastapi  | 95.4%         | 95.4%        | 0.9946 → 0.9946    |
| rich     | 100.0%        | 100.0%       | 0.9964 → 0.9964    |
| faker    | 95.0%         | 95.0%        | 0.9537 → 0.9537    |
| hono     | 88.3%         | 88.3%        | 0.8326 → 0.8326    |
| ink      | 93.3%         | 93.3%        | 0.9905 → 0.9905    |
| **faker-js** | **76.7%** | **93.3%** ✓ | 0.9477 → 0.9477    |

Auto-detect output on each corpus is logged at fit time as
`[auto-asym] cluster_rare probe: …`, including the fire rate and the
keep/disable decision. Operators can audit the choice per repo.

## Consequences

- **Existing calibrations need re-fit** to pick up the new resolved
  values. Older configs without the new keys fall through to
  `cluster_rare_threshold=0` (legacy behaviour) — no crash, no silent
  divergence.
- **faker-js style FP rate is back at the era-13.5 baseline (1.9%)**.
  The rule's catches are paid for by reintroducing those FPs; that's
  the explicit asym tradeoff and the auto-detect probe is what
  validates it per-corpus.
- **Bench reflects production now.** `just bench` exercises the same
  code path users hit. No more "bench shows X but prod ships Y"
  surprises.

## See also

- `docs/research/evidence/era13-final.md` — original era-13.5 G7
  cancellation analysis
- `benchmarks/README.md` §"Era 13.5 — asymmetric calibration"
- `engine/argot/scoring/calibration/__init__.py:main` — the new probe +
  CLI wiring
