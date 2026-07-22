# Accept-time human-brief prototypes

**Issue:** EV-03 · **Status:** fixed research prototypes, not CLI output

The terminal contract is 80 columns and at most 12 lines. Every candidate is
advisory: it preserves evidence, names no finding as a defect, and leaves the
acceptance decision with the human. Exposure/deduplication semantics remain
subject to DR-02; these examples exercise the states EV-03 requires.

## Candidate A — evidence first

### Clean

No output.

### One finding

```text
Argot brief — advisory
1 finding in src/auth/session.ts
error · layering · src/auth/session.ts:42
  imports api/http, reversing the established dependency direction
Evidence: 18 prior imports flow api → auth
Inspect the diff and decide whether this exception is intended.
```

### Many mixed findings

```text
Argot brief — advisory · 3 findings
error · layering · src/auth/session.ts:42
  api/http reverses the established api → auth direction
error · foreign-import · src/jobs/mail.ts:8
  imports axios; no prior repository import
warn · test-weakened · tests/auth.test.ts:91
  assertion was loosened while covered production code changed
Inspect evidence and decide whether to accept, revise, or suppress
intentionally.
```

## Candidate B — action first

### One finding

```text
Argot brief — advisory
Review 1 repository-pattern finding before accepting this change.
error · layering · src/auth/session.ts:42
Evidence: api/http reverses 18 prior api → auth imports
This is a prompt for judgment, not a blocked action.
```

### Many mixed findings

```text
Argot brief — advisory
Review 3 findings: 2 error · 1 warn
error · layering · src/auth/session.ts:42
error · foreign-import · src/jobs/mail.ts:8
warn · test-weakened · tests/auth.test.ts:91
Open the diff and evidence; accept, revise, or suppress intentionally.
```

## Shared edge states

Suppressed, excluded, off, and inline-ignored findings produce no brief; raw
evaluation records retain their state. A stale/unfitted/setup failure is a
separate, non-blocking diagnostic:

```text
Argot integration — advisory setup diagnostic
This repository is not fitted or its model is stale; no finding brief was
produced.
Run `argot init` when ready. The integration did not block your work.
```

## DR-14 structured proxy evaluation

DR-14 may use the following fixed, blinded proxy to choose a layout. It is not
a human-subject study and does not establish usability, comprehension, or
launch readiness; GitHub issue #273 tracks the required human study.

1. Give reviewers randomized, unlabeled A/B samples at exactly 80 columns.
2. For clean, one, many, suppressed, and diagnostic states, score a fixed
   checklist: correct state; blocking vs advisory; strongest severity; evidence
   location; and remaining human action.
3. Record clipping, answer, elapsed time, reviewer role (`fresh` or
   `experienced`), and any uncertainty. Keep the raw randomized order and
   answers.
4. Select only if one candidate has no checklist regression and fewer
   ambiguities; otherwise record no selection and retain both candidates.

This supplies a repeatable decision proxy with explicit limits. It does not
claim that a human study passed or that either prototype has shipped.
