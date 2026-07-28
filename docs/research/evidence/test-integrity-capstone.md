# Test-integrity guard — capstone (definitive numbers + journey)

> **➕ EXTENDED (2026-07-20) — Pascal added as the 12th language.** castle-engine (FPCUnit)
> contributes 11 fixtures across 6 tactics: **11/11 caught, 0/4 controls, gating-FP 1/62 = 1.6%.**
> Fixing two Pascal wiring gaps in `test_inventory/mod.rs` (`tautology_capable` was
> case-sensitive-lowercase → missed PascalCase `AssertEquals`; `defined_symbols` lacked the Pascal
> `defProc`/`declType` node kinds → `test-deleted` couldn't confirm survival) enabled the
> tautologization + test_deletion tactics. mORMot2 is N/A (bespoke `TSynTestCase` framework outside
> the FPCUnit/DUnit convention). New aggregate: **23 corpora / 12 languages · 155/164 = 94.5% catch
> · 0/106 controls · 45/3602 = 1.25% gating-FP.** See [`pascal-language-port.md`](pascal-language-port.md).
> The 11-language canonical run below is the original.
>
> **↻ RE-MEASURED (2026-07-28) — 154/164, not 155.** `just integrity-verify` on
> `fix/status-repo-flag` reports **154/164 = 93,9 %**, with controls unchanged at
> **0/106** and every tactic matching except `test_deletion` (23/25 → 22/25); the
> three misses are hono, laravel and composer. An A/B against the same branch
> *without* that day's grammar fixes reproduces hono 5/7 with the identical two
> misses, so the difference **predates** them and is not attributable to any
> change made that day. Deliberately not bisected further: one fixture, no
> control movement. The claim manifest carries the measured 154/164; this
> document keeps 155 as the number the original run produced.

**Date:** 2026-07-10 · **Branch:** `feat/new-signal` · **Rubric:**
`benchmarks/catalogs/RUBRIC-INTEGRITY.md` (frozen taxonomy + one recorded
amendment) · **Scout evidence:** `test-integrity-feasibility-scout.md`

The `integrity` rule group (`test-deleted` / `test-disabled` /
`test-weakened`, `--features integrity`, ON in releases) catches an AI coding
agent gaming tests to green a failing suite. This memo records the definitive
production-path numbers, the FP-hardening journey, and the honest limits.

## Definitive numbers (one canonical run, frozen config, fresh fits, 22 corpora / 11 languages)

**Catch (gaming fixtures, production fit→check, expected rule fires):
144/153 = 94.1%** (gate ≥ 90% ✓). Per tactic:

| tactic | caught | tactic | caught |
|---|---|---|---|
| body_gutting | 25/25 | tautologization | 20/20 |
| comparison_widening | 20/20 | skip_disable | 21/21 |
| test_deletion | 22/24 | assertion_deletion | 20/22 |
| expected_retarget | 16/21 | | |

**Authored controls (legit refactors/renames/moves/deletions/flaky-skips):
0/102 fired** ✓.

**Accepted-history FP (600 commits per corpus replayed OUTSIDE the fit's
calibration window): gating severity 44/3540 test-touching commits = 1.24%**
(gate ≤ 2% ✓); **any-severity annotation rate 103/3540 = 2.91%** (published,
never hidden — the rubric amendment records why the gate is measured at
gating severity: `test-weakened` ships default `warn`).

**Base guardrail:** `just verify` ✓ with the feature off; ink quick-bench
95.7% novel-pattern catch (baseline unchanged); the feature is a build-time
gate — base binaries contain none of this code.

**Per-corpus gate (≥85%):** met by 21/22 corpora (every corpus ≥ 6/7 =
85.7%) — **except excalidraw 5/7 (71.4%)**, the honest limit below.

## Design that survived (why FP is this low)

1. **Scope guard** — no rule fires on tests-only changesets.
2. **Per-repo learned gates** (fit-time mini-replay of 150 accepted
   commits, `.argot/integrity.json`): an event class this repo's normal
   development trips ≥2× at >2% is disabled *for that repo*. Retarget is
   off unless the history shows zero isolated flips.
3. **Event refinements**: changeset-wide move/replacement detection,
   definition-survival for deletions, prod net-growth requirement, pure
   excision (multiset-aligned), pure-literal-subject tautologies
   (core-vocabulary callees only), one widening event per site key,
   bulk-sweep guard (>3 affected tests = migration), positional pairing of
   same-named tests (JUnit `@Nested`), extract-to-helper excuses
   (verbatim count-aware + added-test-line word coverage).

Every refinement was driven by a named false positive from real accepted
history or an authored control, and each carries a regression test
(`scoring/integrity.rs`); the FP loop ran under zero-catch-regression —
the two fixture regressions it briefly caused (gh-cli excision via
positional-tail mispick; guava widening via duplicate events tripping the
bulk guard) were diagnosed to root cause and fixed, not tuned around.

## Honest limits (documented, not papered over)

- **expected_retarget (16/21)** — a bare literal retarget is statically
  indistinguishable from healthy TDD updating an expectation; the scout
  measured 7–12% natural isolated-flip rates. It fires only in repos whose
  history shows none (rich, eslint, ripgrep, junit5, rocksdb-class repos);
  elsewhere the per-repo gate keeps it silent, and the fixture misses in
  fastapi/hono/express/powershell/homebrew/excalidraw are that gate working.
  Do not re-chase without new information.
- **excalidraw (5/7)** — its accepted history genuinely weakens/skips tests
  alongside prod changes at 3–5% (per-event, ≥2 observations); enabling those
  events there would trade directly into FP. The per-corpus catch gate fails
  for this one corpus by design priority (FP first). 
- **Per-corpus gating-FP tails** (hugo 3.8%, junit5 4.8%, rocksdb 3.1%,
  jellyfin 4.3% of their test-touching commits): surgical test deletions and
  guttings in huge, fast-moving repos that survive every static refinement —
  reviewable-looking events a human would also flag from the diff alone. The
  overall gate holds (1.24%); per-corpus rates are published.
- **C** — no universal framework; curl/redis covered via harness-visible
  subsets (deletion/gutting rich, some tactics N/A — see the catalogs).
  Plain-Go (hugo) cannot express walker-visible tautology/widening (t.Error*
  truth lives in if-guards); recorded N/A in its catalog.
- **Fraudulent test additions** (tests asserting wrong behaviour) are out of
  scope — needs an oracle, not a diff.

## Release decision

Shipped in release binaries (`dist-workspace.toml` `features += integrity`).
Default severities, justified by the replay: **`test-deleted` error** (0.85%
of accepted test-touching commits carry one), **`test-disabled` error**
(0.42%), **`test-weakened` warn** (0.99% of commits carry a weakened
annotation; the warn tier accounts for the 1.67-point gap between the 2.91%
any-severity and 1.24% gating rates — reported, never fails `check`; users
can promote it per repo). Confidence
pinned `suspicious`. No user tuning knobs; the `[rules]` severity surface is
the only control.

## Reproduction

- `just integrity-verify` — fixture-recall + control guard (fits cached; the
  fast regression check when the detector or walkers change).
- `just bench-integrity-fp` — the accepted-history replay (600 commits per
  corpus outside the calibration window, dual-tier rates).
- Fixtures: `benchmarks/catalogs/<corpus>/integrity_fixtures.yaml` (22
  catalogs authored by parallel executors against pinned SHAs, each
  0-grounding-error verified).

## Journey (chronological, for archaeology)

1. Scout (4 corpora): raw event rates hopeless (retarget 35–60%!); refined
   discriminators → ~0–1%; retarget declared ungatable-by-default.
2. Engine + walkers (11 languages) + fixture wave (22 corpora, 11 agents):
   first full run 90.8% catch / controls 0.
3. FP replay exposed: bulk migrations (junit5 ×118-test sweeps), custom
   assert helpers as false tautologies (rocksdb `assertRunFAIL`), positional
   misalignment on colliding site keys (3 classes), name collisions
   (`@Nested`), prod-side vocabulary leaking into move excuses, marker lines
   outside test spans (Rust `#[ignore]`), single-observation gates.
4. Each fixed with a regression test; severity split recorded in the rubric
   amendment; canonical run: **94.1% catch / 1.24% gating FP / 0 control FP**.
