---
name: argot-plan
description: Plan a piece of argot work — research era, product feature, refactor, anything substantial. Runs a grill-me interview to surface constraints, drafts an outline, gets explicit user approval, then writes a PRD to `.scratch/<slug>/PRD.md`. NEVER implements; planning only. Use when the user says "plan X", "write plan", "PRD for X", "new era", or invokes /argot-plan.
---

# argot-plan

Produces a `.scratch/<slug>/PRD.md` for the work the user wants planned.

## Hard rules

1. **Plan only — never implement.** This skill stops at writing the PRD. Even after the PRD is written, do not start editing source code, deleting files, or running migrations until the user explicitly says "go" or "execute" in a separate message. If the user starts asking implementation questions during the interview, defer them to the PRD.
2. **Output path is `.scratch/<slug>/PRD.md`.** Always. Even for research-era plans (which historically lived at `docs/research/era-N-hypotheses.md`) — the PRD starts in `.scratch/` and is moved/canonicalized later if needed. `<slug>` is kebab-case and short (e.g. `cli-cleanup`, `era-14`, `multi-language-adapter`).
3. **Never write the PRD before user approval.** Steps 1–3 happen in conversation; only Step 4 writes the file.

## Step 1 — Read the relevant existing record

Before asking anything, read the docs that bound the planning space.

**For all plans:**
- `CLAUDE.md` — current architecture, conventions, hard rules
- `.scratch/` — check for existing PRDs that overlap or supersede this work
- The relevant source tree for whatever the user is planning to change (skim, not deep-dive)

**Additionally, if this is a research-era plan:**
- `docs/research/README.md` — current baseline, timeline, gate clearance history
- Every `docs/research/NN-*.md` era narrative — headline findings and failure reasons
- The latest `docs/research/era-*-hypotheses.md` — open residuals, "Do Not Retry" table
- `docs/research/evidence/` — skim every title; open any that are relevant to candidate hypotheses

Build a mental "already tried" / "already decided" index. Any approach that appears in a "Do Not Retry" table or a failed evidence doc **must not appear in the new plan** unless there is an explicit, documented reason why the conditions have changed. Same for product decisions already settled in CLAUDE.md or prior PRDs.

## Step 2 — Grill-me interview

Ask one question at a time. For each, give your recommended answer so the user can confirm or redirect rather than answer from scratch.

Pick the branches that fit the work. Skip any already answered by the docs.

**Branches for any plan:**
1. **Problem** — what's broken, missing, or about to break? Why now?
2. **Scope** — what's in, what's explicitly out, what's deferred?
3. **Constraints** — hard rules that bound the design (project conventions, API stability, performance, deadlines)
4. **Decisions** — discrete forks where the user must pick: A or B? Each presented with your recommendation + the main tradeoff
5. **Risks** — what could go wrong, what's reversible, what's not
6. **Execution shape** — one PR or split? Order of changes? Anything that must ship together?

**Additional branches if this is a research-era plan:**
7. **Hypothesis space** — what are the candidate signal classes? Confirm none appear in any "Do Not Retry" table or failed evidence doc; if they do, explain what has changed
8. **Pre-registered gates** — recall target, per-corpus FP ceiling, regression clause, corpus-specific floors
9. **Phase structure** — how many phases, in what order, which can run in parallel, which are precondition gates
10. **Bench cost** — for each phase: dirty script, scoped bench, or full corpus? Default to cheapest signal first
11. **Conflict check** — do any phases touch the same production files?
12. **Ceilings** — architectural ceiling, per-phase EV, fragile catches

Stop after each answer and move to the next branch.

## Step 3 — Draft outline

After the interview, post a one-page outline in chat:
- Working title and slug
- Problem (2–3 sentences)
- Decisions taken (one line each)
- Steps / phases (one line each, with rough scope)
- Risks / call-outs
- Anything still open

Wait for explicit user approval before writing the file.

## Step 4 — Write the PRD

File: `.scratch/<slug>/PRD.md`. Create the directory if it doesn't exist.

**Required sections (any plan):**

```
# <Title> — PRD

Status: planning

## Background
[why this work exists, what's broken or missing, what's the trigger]

## Decisions (locked by user)
[table: # | decision | choice — captures the forks resolved during interview]

## Step 1 — <name>
[what changes, files touched, behavior delta]

## Step 2 — <name>
...

## Risks / call-outs
[reversibility, behavior changes users will notice, what's NOT fixed]

## Execution order
[numbered list — even for a single PR, ordering matters for clean diffs]

## Open questions during implementation (not blocking plan approval)
[questions to resolve while coding, not before]
```

**Additional sections if this is a research-era plan:**

```
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

**After writing:** post a one-line summary to chat (`Plan written to .scratch/<slug>/PRD.md`) and stop. Do not start implementation. Do not create implementation tasks. Wait for the user to say "execute" or "go".

## Constraints (always apply, never negotiate)

These bind any plan that touches argot's production code:

- **No hardcoded domain knowledge** in `engine/argot/scoring/` or `cli/src/` — no framework names, no corpus-specific literals. Tests/eval may use them.
- **Language and corpus agnostic** — prod scorers must work on any repo, not just the bench corpora.
- **No era/phase labels in prod symbols** — class/file/function names use domain concepts only.
- **Behaviour-first unit tests** — assert on outputs for given inputs; tests survive semantics-preserving refactors.
- **Bench cost ladder** (research only) — dirty experiment script → scoped bench (1–2 corpora) → full corpus. Full corpus = final confirmation only.
- **Evidence always kept** (research only) — `docs/research/evidence/` gets a doc for every experiment regardless of outcome. Experiment scripts are cleaned up after.
