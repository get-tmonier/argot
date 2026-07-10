# Test-integrity guard — Phase-2 feasibility scout (evidence memo)

**Date:** 2026-07-10 · **Branch:** `feat/new-signal` · **Scout:**
`benchmarks/scout-integrity/` (dirty standalone crate, not a workspace member;
deleted after this memo per research-workflow policy — the numbers survive
here). Taxonomy under test: `benchmarks/catalogs/RUBRIC-INTEGRITY.md` (frozen
before scoring).

## Questions

1. Can a cross-language test inventory (test fns, assertion sites, skip
   markers, expected literals) be extracted with the pinned tree-sitter
   grammars? 2. What are the NATURAL base rates of each gaming-taxonomy event
   in accepted history — i.e. can the rules gate at FP ≤ 2% of accepted
   test-touching commits?

## Method

Standalone Rust scout (pinned tree-sitter 0.23 grammars, git2). Corpora:
fastapi (Python), hono (TS), ripgrep (Rust, in-file unit tests), gh-cli (Go).
600 first-parent commits each (merges skipped, matching `git_walk`). Per
commit: classify changed files test/prod (path conventions), extract old+new
inventories of changed test files, diff into taxonomy events. Every event
counted twice: RAW (naive definition) and REFINED (production discriminators,
iterated over 5 scout rounds against ground-truth samples).

## Result 1 — extraction works

| corpus | test files | tests | assertions | pre-existing pure-literal assertions |
|---|---|---|---|---|
| fastapi | 493 | 2,166 | 4,490 | 0.13% |
| hono | 126 | 2,411 | 5,421 | 0.35% |
| ripgrep | 39 | 449 | 1,711 | 0.06% |
| gh-cli | 349 | 1,556 | 7,385 | 0.84% |

Assertion-strength tiers (exact > relational > existence), literal
fingerprints, and skip markers all extract cleanly; Rust in-file `#[test]`
functions are found inside prod files (test identity must be per-function,
not per-file, for Rust).

## Result 2 — raw events are hopeless; refined discriminators work

Rates = % of accepted prod+test commits exhibiting the event
(raw → refined):

| event | fastapi | hono | ripgrep | gh-cli |
|---|---|---|---|---|
| skip_added | 0% | 0% | 0% | 0.3% |
| body_gutted | 0% | 0% | 0% | 0% |
| tautologized (pure-literal subject) | 0% | 0.4% | 0% | 0.6% |
| assertion_removed | 0→0% | 22.4→**0%** | 0% | 2.7→**0.9%** |
| comparison_widened | 0% | 5.7→**0%** | 0% | 0.6→0.3% |
| test_file_deleted | 14.7→**0%** | 1.2→0.4% | 0% | 5.3→**0%** |
| test_deleted | 2.9→**0%** | 6.1→**0.8%** | 0% | 4.7→**1.8%** |
| expected_retarget | 35.3→**11.8%** | 60.2→1.6% | 5.4→0% | 40.7→**7.1%** |

**UNION FP (all refined rules except retarget, scope guard active):
fastapi 0.00% · hono 1.45% · ripgrep 0.00% · gh-cli 3.67%.**
Including isolated-retarget: 9.09% / 2.90% / 0.00% / 10.45%.

## The discriminators that earned their keep (production requirements)

1. **Scope guard** (definitional): no rule fires on tests-only changesets.
2. **Changeset-wide move/replacement detection**: a deleted test whose name
   reappears anywhere, or whose body (word-set Jaccard ≥ 0.3) matches an added
   test anywhere, is a migration. Halved deletion FP (hono 6.1→3.3).
3. **Definition-survival**: deletion only counts when defined prod symbols the
   test exercised (∩ old prod defs) still exist in the post-image. Word-level
   overlap was useless (passed everything); definition-level cut hono 3.3→1.2,
   gh-cli 3.2→2.7. Production: adapters' `callable_definitions`.
4. **Prod net-churn direction**: deletion/removal events only count when the
   prod side is net-growing. Ground truth showed the surviving legit deletions
   were code removals/externalizations (gh-cli 8016244 go-gh prompter,
   41a4571 variable-list rework, both heavily net-negative). gh-cli
   removed_ref 2.1→0.9, deleted_ref 2.7→1.8.
5. **Pure excision** (assertion removal): fires only when every surviving
   assertion of the test is unchanged (sites + literals) — the diff to the
   test is nothing but the removal. hono 22.4→0.
6. **Pure-literal subject** (tautology): assertion whose subject has zero
   identifiers. Precise (0–0.6% natural; earlier text heuristics flagged real
   `toHaveBeenCalledWith` calls).
7. **Site-matched widening**: an exact-tier site vanished AND a weaker-tier
   site with overlapping words appeared in the same test (word Jaccard ≥ 0.4).
   hono 5.7→0.

## Verdicts

- **Gatable at error by default (≈0–1% natural):** skip_disable, body_gutting,
  tautologization, comparison_widening, assertion_deletion (pure excision),
  test_file_deletion. These six carry the catch headline.
- **Gatable with per-repo calibration:** test_deletion (0–1.8%; repos with
  high measured deletion churn need a fit-time-learned downgrade — the
  semantic-layer mini-replay pattern). gh-cli's 3.67% union means per-repo
  calibration is REQUIRED, not optional, to hold the ≤2% gate everywhere.
- **NOT gatable as a standalone default rule:** expected_retarget. Even the
  strictest static definition (1–2 isolated flips, nothing added anywhere)
  fires on 7–12% of accepted commits in fastapi/gh-cli — updating an expected
  literal after an intentional behaviour change is what healthy TDD looks
  like, and it is statically indistinguishable from gaming (both make
  expected == what the new code emits). Design decision: retarget events ship
  inside `test-weakened` only where the repo's own history calibrates them
  quiet (ripgrep-like repos), otherwise they are suppressed at fit. The
  Phase-5 bench must report retarget catch honestly per corpus — expected to
  be the weak cell, documented, never inflated.

## Carried into Phase 3

Engine needs: old+new blob pairs incl. deletions (new cfg-gated delta
collection beside `PatchBatch`); per-function test identity (Rust);
changeset-level event refinement (not per-file); fit artifact with learned
test paths, assertion idioms, and per-event accepted-history base rates
driving per-repo gates; all thresholds internal.
