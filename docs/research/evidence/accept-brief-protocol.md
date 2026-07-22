# Combined-brief evaluation protocol

**Issue:** EV-02 · **Status:** protocol, not a measurement · **Date:** 2026-07-22

This protocol reserves no product behavior and sets no release threshold. DR-02
must select the exposure policy before a harness or a reported combined rate is
used as evidence. The proposed defaults below are explicit placeholders so a
future implementer does not silently choose metrics, sampling, or labels.

## Policy inputs DR-02 must freeze

| Input | Proposed evaluation value | Requires DR-02 confirmation |
| --- | --- | --- |
| Eligible finding | Enabled, unsuppressed finding from the released detector composition | Yes |
| Display order | Error before warn; then rule, path, and line in stable lexical order | Yes |
| Brief | One advisory summary for one changed state | Yes |
| Clean state | No brief | Yes |
| Dedupe | Repository, base/head, diff, effective config/model fingerprint, and sorted finding hashes | Yes |
| Repeat window | 24 hours | Yes |
| Failure behavior | At most one non-blocking diagnostic per repository session | Yes |
| Pre-write hook | Excluded: it is a separate, narrow Claude pre-write ask | Yes |

Until those values are approved, this document is a reproducible protocol
candidate, not an implementation instruction or public claim.

## Unit, sampling, and adjudication

The unit is one accepted repository change: a merge commit or accepted
non-merge commit with a reproducible base and head. Build the candidate list
from a pinned repository/revision window before scoring, recording every
exclusion. Sort candidates by repository, timestamp, and SHA; stratify by
language and changed-file-size bucket; sample with a published PRNG seed. Do
not replace a selected change after seeing its findings.

For every selected change, fit or restore only the base-state model, scan the
base-to-head diff with the recorded configuration, and preserve raw output and
timings. Two blinded adjudicators classify each displayed hit as
`actionable`, `not-actionable`, or `uncertain`, each with an evidence note. A
third recorded reconciliation resolves disagreement while retaining both
original labels. This is classification of displayed findings, not a claim
that a detector proved a defect.

Keep these denominators separate: accepted changes; findings; displayed hits;
briefs; diagnostics; suppressed findings; and execution errors. Never divide a
detector-specific quantity by the full-brief population.

## Required raw record

One JSON object is required per accepted change:

```json
{"schema":1,"repo":"owner/name","repo_revision":"sha","base":"sha","head":"sha","accepted_unit":"merge|commit","sampling":{"population_id":"","stratum":"","seed":0,"selected":true},"environment":{"argot_version":"","features":[],"config_fingerprint":"","model_fingerprint":""},"timing_ms":{"fit":0,"scan":0,"render":0,"total":0},"counts":{"findings":0,"displayed_hits":0,"briefs":0,"diagnostics":0},"findings":[{"hash":"","rule":"","severity":"","path":"","line":0,"suppressed":false}],"adjudication":[{"hash":"","rater_a":"","rater_b":"","final":"","note":""}],"status":"ok|setup_diagnostic|execution_error","raw_output_path":""}
```

Report findings/accepted change, displayed hits/accepted change,
briefs/accepted change, adjudication proportions (including `uncertain`),
clean/noisy p50 and p95 timing, repeat-brief rate, and diagnostic/error rate.
DR-03, not EV-02, owns pass/fail thresholds.

## Dry-run receipt

Before harness implementation, create three fixture records: clean; one
unsuppressed error plus warn; and one only-muted/inline-ignored finding. By
hand, the findings/displayed-hits/briefs totals must be respectively `0/0/0`,
`2/2/1`, and `1/0/0`. Retain each record even when it creates no brief. This
tests the schema and denominators; it does not measure production noise.

## Reproduction

The future benchmark owner should place the fixture manifest under the BM-06
lease, validate the JSON objects against this schema, and publish raw records
with the selected corpus/revision/seed. No benchmark run or numeric claim is
created by this protocol.
