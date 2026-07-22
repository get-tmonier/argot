# Combined briefing evaluation — deferred

**Issues:** BM-06, BM-07, BM-08, BM-09
**Date:** 2026-07-22
**Status:** unmeasured / deferred; this is not a passing gate result.

## What is reproducible now

`argot-bench --mode accept-brief` calls `argot_core::check::run_check`, the
same composition facade used by the distributed binary. Its accepted-change
records preserve the EV-02 fields, and the pinned three-record protocol sample
is at `benchmarks/accept-brief/dry-run-records.jsonl`.

The sample is only a denominator and aggregation receipt. Its hand-checked
totals are: three accepted changes, three findings, two displayed hits, and
one advisory brief. It contains no measured repository, no adjudication of a
real accepted change, and no combined-noise result.

## Why the full evaluation is deferred

DR-03 requires at least 1,000 pinned accepted changes from at least 10
repositories and five supported languages, blinded adjudication, preserved raw
records, declared latency hardware, and mechanical evaluation of every gate.
Those corpus and adjudication inputs are not present in this checkout. The
single Very High artifact-producing evaluation has not been scheduler-granted
or run.

Accordingly BM-09's DR-03 outcome is **defer**. No automatic behavior is
shipped and no low-noise, pass, latency, or combined-rate claim is enabled.

## Required next operation

After the scheduler confirms the Very High lease, provide the frozen manifest
and adjudication corpus, then run the release-composition benchmark build. Keep
the resulting raw JSON, `records.jsonl`, `combined.json`, hardware receipt, and
all unfavorable records. The result must select DR-03 pass, fail, latency-only
defer, or evidence-inconclusive defer mechanically; it must not tune detectors
or replace sampled changes.
