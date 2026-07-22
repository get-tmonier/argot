# PR-15 work log — release-readiness automation and evidence

**Branch:** `codex/pr-15-release-readiness-20260722t1635`
**Base:** `origin/main` `cf6ba445`
**Date:** 2026-07-22

## ON-01 — deterministic audit-to-habit journey

- **Goal:** exercise a no-state Linux journey: install/binary, `audit`, `init`,
  `check`, a chosen integration, a finding, a reasoned mute, and a clean rerun.
- **Lease:** new end-to-end/release fixture and script following existing test
  conventions.
- **Exclusions:** five-platform matrix and every agent integration.
- **Dependencies:** AU-03, CLI-08/09, the selected PC-02 or PL-05 integration,
  and AS-01.
- **Acceptance:** starts with no `.argot`, documents every mutation and network
  action, and ends in the expected suppressed/clean state.
- **Validation:** deterministic automated Linux run.

## QA-01 — supported-platform clean-install journeys

- **Goal:** extend ON-01 across published installer targets.
- **Lease:** reusable workflow coverage and CI/manual receipts.
- **Exclusions:** other agents and load testing.
- **Dependencies:** ON-01, CI-02–05, and PL-06.
- **Acceptance:** every claimed target has a passing journey, or its claim is
  removed as an explicit unverified limitation.
- **Validation:** CI matrix plus a manual receipt where coverage is interactive.

## QA-02 — repository-wide public claim audit

- **Goal:** classify public occurrences against CL-01.
- **Lease:** final audit report under `docs/execution/` only.
- **Exclusions:** product, documentation, landing, README, strategy, and backlog
  edits; small fixes return to their owning issue.
- **Dependencies:** all public-area PRs merged and CL-01.
- **Acceptance:** no forbidden current claim remains; every exception identifies
  path, reason, and owner; unshipped lifecycle language is future tense.
- **Validation:** repository grep, image/OCR or manual inspection, and generated
  output samples.

## REL-01 — compatibility and migration notes

- **Goal:** document pre-commit, JSON, human-output, Action metric, and plugin
  lifecycle changes for existing users.
- **Lease:** migration/release-note source and GitHub release-note template/input.
- **Exclusions:** hand-maintained per-version changelog duplication.
- **Dependencies:** DR-13 and final behavior PRs.
- **Acceptance:** every compatibility change includes before/after, required
  action, and a canonical documentation link.
- **Validation:** upgrade a prior-release fixture by following the instructions.

## REL-02 — release version-consistency check

- **Goal:** make Cargo, plugin, skills, MCP registry, npm/site version, and Action
  tag agree before publication.
- **Lease:** `.github/workflows/release.yml` or `auto-release.yml` and a small
  version-check script/test.
- **Exclusions:** release cadence and version-policy changes.
- **Dependencies:** PL-06 and CI-02–05.
- **Acceptance:** seeded mismatch fails before publish; a consistent tree passes.
- **Validation:** release workflow dry run/fixture.

## Release constraint

REL-03’s automatic lifecycle is deferred. This PR must not introduce it or
describe it as shipped current behavior.

## Recovery status — 2026-07-22

The recovered REL-02 commit is retained. The remaining leased artifacts cover
the following acceptance work without changing product, consumer copy, or
strategy files:

- **ON-01:** the isolated Linux fixture starts without `.argot`, records the
  offline network branch and every mutation, audits, initializes, produces a
  finding, mutes it with a reason, and verifies the suppressed clean rerun.
- **QA-01:** the reusable matrix installs a candidate binary into an isolated
  prefix on the three runner families that cover the five published targets;
  the journey checks the offline branch and `uninstall --dry-run` ownership.
  Published-release installer download/update behavior remains owned by the
  release workflow and requires the final Very High CI/release receipt.
- **QA-02:** `PR-15-claim-audit.md` records the prescribed grep, manual media
  review, generated-output samples, classifications, and exceptions. Lifecycle
  language remains future-only.
- **REL-01:** `release-migration.md` supplies before/after/action/link entries
  for pre-commit, JSON, human output, Action metrics, plugin lifecycle, and
  rollback/opt-out. `.github/release.yml` is the GitHub release-note input.
- **REL-02:** the committed checker compares Cargo, plugin, registry/npm, site,
  and tag metadata; its fixture proves both consistent and seeded-mismatch
  cases.
