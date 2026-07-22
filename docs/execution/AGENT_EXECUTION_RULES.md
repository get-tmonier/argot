# Argot autonomous-agent execution constitution

These rules are mandatory for every coding, documentation, research and QA
agent executing the Argot backlog. The issue body defines the work; this file
defines how work is performed.

## 1. Authority

- Strategy and product decisions are frozen. Never edit `FOUNDER.md`, any
  `docs/strategy/*` file or reinterpret a standing decision.
- Never edit `GITHUB_BACKLOG.md`, `PR_PLAN.md`, `BATCH_PLAN.md` or this file
  during execution. Named new execution artifacts are allowed only when an issue
  explicitly requires them.
- An unresolved `DR-*` issue is a hard gate. Do not choose product semantics on
  its behalf.
- The live issue body and its acceptance criteria are immutable. Execution may
  complete, block or split it—never rewrite, merge, redefine or broaden it.

## 2. One owner, one worktree, one PR

- Work only in the new worktree and branch assigned to this PR.
- Never enter, read uncommitted state from or modify another agent’s worktree.
- Never reuse a worktree path or branch from an earlier PR.
- Start from the latest merged `origin/main`. Never base work on another agent
  branch or open PR. Open dependencies do not count as complete.
- Touch only the PR’s leased paths. Never touch a file owned by another active
  or future PR unless the lease explicitly includes it.
- Do not delegate subparts to agents that would write into the same worktree.

## 3. Execute issues literally

- Work on one issue at a time, in dependency order.
- Before editing, restate the issue goal, leased files, exclusions, dependencies,
  acceptance criteria and validation in the work log.
- Make the smallest change satisfying the issue. Do not redesign adjacent APIs,
  refactor unrelated code, clean up nearby code or “improve while here.”
- Never fix an unrelated defect opportunistically. Record evidence and create a
  follow-up issue for the correct owner.
- Never broaden support, compatibility, platform, detector or public-claim scope.
- Preserve deterministic output, stable ordering, schemas, snapshots and fixture
  provenance unless the issue explicitly changes their contract.

## 4. Stop instead of guessing

Stop the current issue and mark it blocked when:

- a dependency or decision is unresolved;
- the required edit is outside the file lease;
- current repository behavior contradicts the issue’s premise;
- acceptance criteria cannot be met deterministically;
- work will exceed approximately 90 focused minutes or 20 files;
- the PR is likely to exceed 60 files or 2,000 reviewed lines;
- a new compatibility, default, confidence, blocking or public-claim decision is required;
- unrelated user changes overlap the leased files.

Do not silently reinterpret the issue. Post the evidence. If the issue must be
split, preserve its body, block it with a comment and create new child issues at
testable seams.

## 5. Validation and CI

- Run the issue’s focused local validation immediately after its change.
- Run the PR’s consolidated local validation only after all included issues pass.
- Never request expensive CI merely to discover a local compile, format, fixture,
  link, schema or snapshot failure.
- Request CI once when the PR is locally ready. Rerun only for a relevant fix or
  documented infrastructure flake.
- Only the scheduler grants the Very High CI lease. Never start a Very High run
  while another holds it.
- Do not weaken, skip or delete a test to make CI green unless the issue explicitly
  requires that exact test-contract change.
- An out-of-lease CI failure blocks the PR and becomes a new issue for its owner.

## 6. Claims and product boundaries

- Never market future, conditional, prototype or user-wired behavior as shipped
  automatic behavior.
- Keep audit as the acquisition front door and awareness at acceptance as the
  product job.
- Preserve the free open-source individual local core, no required account/cloud,
  no default telemetry and user-owned portable configuration.
- Preserve the non-generative authoritative analytical path.
- Treat findings as prompts for human judgment, not proof of defects.
- Numeric, platform, privacy and integration claims must come from the approved
  claim/capability sources and retain their qualifiers.

## 7. Commits and handoff

- Prefer one reviewable commit per completed issue, prefixed with its stable ID.
- Never commit unrelated formatting, generated churn, editor state, secrets or
  another agent’s changes.
- The PR description lists every included issue and, for each one: acceptance
  result, validation command/result and public-claim/compatibility impact.
- Include generated-asset provenance, benchmark raw data, screenshots or manual
  receipts when the issue requires them.
- Do not merge your own PR unless the scheduler explicitly assigns merge authority.
- After merge, the scheduler records the merge SHA, closes completed issues and
  permanently destroys the worktree. Agents do not retain or repurpose it.

## Start checklist

```text
[ ] All prerequisite PRs are merged into latest origin/main
[ ] All decision gates are resolved
[ ] Dedicated new branch and never-used worktree assigned
[ ] Exactly one owner and file lease recorded
[ ] No active PR owns an overlapping path
[ ] Issue bodies and validation read completely
[ ] Local repository instructions read
```

## Completion checklist

```text
[ ] Every included issue is complete, blocked or explicitly split
[ ] No leased-path violation or unrelated change exists
[ ] Focused and consolidated local validation passes
[ ] CI class and run evidence recorded
[ ] PR stays under file/line caps or was split at an issue boundary
[ ] Compatibility and public-claim impact is explicit
[ ] Follow-up discoveries exist as new issues, not hidden scope
```
