# PR-07 work log — combined briefing evaluation

**Branch:** `codex/pr-07-bench-20260722t1334`  
**Base:** `origin/main` `223bed8e913d9fe2e1b8a17c94df6bbed15f0af3`  
**Date:** 2026-07-22

## BM-06 — production-composition adapter

- **Goal:** run the distributed detector composition from the benchmark harness.
- **Lease:** `crates/argot-bench/` and only a minimal approved composition seam if it proves necessary.
- **Exclusions:** no corpus replay, aggregation, public copy, detector tuning, or unrelated detector code.
- **Dependencies:** EV-02 and CLI-02 are merged; DR-02's approved policy requires all enabled, unsuppressed error and warn findings to be displayed advisory evidence.
- **Acceptance:** one deterministic fixture agrees with release-feature `argot check` for the same base/head/configuration.
- **Validation:** release-composition parity test.

## BM-07 — accepted-change replay input

- **Goal:** load pinned accepted changes plus adjudication and emit raw finding records through BM-06.
- **Lease:** `crates/argot-bench/` and pinned benchmark fixtures/results.
- **Exclusions:** no aggregate metrics/full corpus, public copy, threshold tuning, or detector changes.
- **Dependencies:** BM-06.
- **Acceptance:** a deterministic three-case protocol receipt retains repository, base/head SHA, rule, severity, and timing fields.
- **Validation:** EV-02's clean, error+warn, and only-suppressed dry run.

## BM-08 — combined-brief aggregation

- **Goal:** aggregate per-rule/union findings, displayed hits, briefs, latency, and true/false/uncertain adjudications.
- **Lease:** `crates/argot-bench/` and the result schema.
- **Exclusions:** no full evaluation, threshold changes, public copy, or detector tuning.
- **Dependencies:** BM-07.
- **Acceptance:** hand-worked fixture totals match; a changed rule exposes its marginal union contribution.
- **Validation:** known-count unit tests and independent aggregation over the pinned dry-run records.

## BM-09 — frozen evaluation outcome

- **Goal:** publish raw records, aggregate report, and DR-03 verdict for the frozen protocol.
- **Lease:** pinned corpus/results and dated combined evidence.
- **Exclusions:** no corpus selection after results, tuning, automatic behavior, or public combined-noise claim.
- **Dependencies:** BM-08 and the binding DR-03 thresholds. DR-03 remains open only until BM-09 selects an outcome; its policy is approved and permits evidence collection.
- **Acceptance:** a full result would require at least 1,000 accepted changes across at least 10 repositories and 5 languages, all required adjudication/latency denominators, and a mechanical gate verdict.
- **Validation:** deterministic subset rerun and independent aggregate recomputation. The full corpus is the one scheduler-granted Very High CI run and will not be dispatched before local replay/parity/aggregation pass and scheduler confirmation.
- **Current status:** deferred/unmeasured until the pinned corpus, blinded adjudication, and scheduler-granted full evaluation are available. This PR must not fabricate a passing result or ship automatic behavior.
