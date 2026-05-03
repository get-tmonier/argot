---
name: argot-plan
description: Plan a new argot research era. Runs a grill-me interview to surface all constraints, then writes docs/research/era-N-hypotheses.md following project conventions (pre-registered gates, phases, parallelisable units, EV estimates, conflict check). Use when starting a new era or when user says "plan era", "write era plan", "new era".
---

# argot-plan

Produces a `docs/research/era-N-hypotheses.md` for the next research era.
Never writes the plan until the interview is complete and the user has approved the outline.

## Step 1 — Read the full research record first

Before asking anything, read **all** of:
- `docs/research/README.md` — current baseline, timeline, gate clearance history
- Every `docs/research/NN-*.md` era narrative — headline findings and failure reasons
- The latest `docs/research/era-*-hypotheses.md` — open residuals, "Do Not Retry" table
- `docs/research/evidence/` — skim every title; open any that are relevant to the candidate hypotheses

Build a mental "already tried" index. Any approach that appears in a "Do Not Retry" table or an evidence doc marked as failed **must not appear in the new plan** unless there is an explicit, documented reason why the conditions have changed. If you are tempted to propose something that looks like a past experiment, check the evidence doc first and cite why it is different this time.

## Step 2 — Grill-me interview

Ask one question at a time. For each, give your recommended answer so the user can confirm or redirect rather than answer from scratch.

Cover these branches in order, skipping any already answered by the docs:

1. **Unsolved problem** — which residuals / fixture buckets are we targeting? Are we still in the same problem space or pivoting?
2. **Hypothesis space** — what are the candidate signal classes? For each one, confirm it does not appear in any "Do Not Retry" table or failed evidence doc. If it does, explain what has changed. Never propose an approach just because the user hasn't mentioned it was tried — you already read the record.
3. **Pre-registered gates** — recall target (fixture-count and per-category), per-corpus FP ceiling, regression clause, any corpus-specific floor?
4. **Phase structure** — how many phases, in what order? Which can run in parallel? Which are precondition gates for later phases?
5. **Bench cost** — for each phase: dirty script enough, or scoped bench, or full corpus? Default to cheapest signal first.
6. **Conflict check** — do any phases touch the same production files? If yes, flag for orchestrator to handle directly.
7. **Ceilings** — architectural ceiling (what can't any phase reach?), per-phase EV, fragile catches?
8. **Hard constraints** — remind and confirm: no hardcoded domain knowledge in prod, language/corpus-agnostic, no era/phase labels in prod symbols, behaviour-first unit tests.

Stop after each answer and move to the next branch.

## Step 3 — Draft outline

After the interview, post a one-page outline:
- Era N — working title
- Unsolved problem (2–3 sentences)
- Pre-registered gates (table)
- Phases (name, objective, parallelisable?, bench cost, EV)
- Annotated concerns with recommendations

Wait for explicit user approval before writing the full doc.

## Step 4 — Write the plan doc

File: `docs/research/era-N-hypotheses.md` (increment N from latest).

Required sections (follow the structure of existing era docs):

```
# Era N — <title>

## The Unsolved Problem
[baseline table: corpus | recall | FP | uncaught]
[fixture bucket table: shape | count | mechanism | existing infra?]

## What's Been Tried — Do Not Retry
[table: approach | era | outcome | why it can't reach residuals]

## Goals (pre-registered)
[gate table: G1 recall, G2 FP, G3 no-regression + sub-clauses, corpus floors, threshold stability, G-N domain-knowledge]
[recall ceilings: architectural, per-phase, fragile]

## Phases

### Phase N — <name> (<time estimate>)
[what it tests, tasks numbered, decision rule, expected EV]
```

## Constraints (always apply, never negotiate)

- **No hardcoded domain knowledge** in `engine/argot/scoring/` — no framework names, no corpus-specific literals. Tests/eval may use them.
- **Language and corpus agnostic** — prod scorers must work on any repo, not just the bench corpora.
- **No era/phase labels in prod symbols** — class/file/function names use domain concepts only.
- **Behaviour-first unit tests** — assert on outputs for given inputs; tests survive semantics-preserving refactors.
- **Bench cost ladder** — dirty experiment script → scoped bench (1–2 corpora) → full corpus. Full corpus = final confirmation only.
- **Evidence always kept** — `docs/research/evidence/` gets a doc for every experiment regardless of outcome. Experiment scripts are cleaned up after.
