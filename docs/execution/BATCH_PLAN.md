# Argot autonomous execution batches

This is the launch schedule for `PR_PLAN.md`. It maximizes safe parallelism under
the stronger requirement of zero shared ownership. A batch begins only from
merged `main`; no worktree for a later batch exists while its prerequisite PR is
open.

All agents receive `AGENT_EXECUTION_RULES.md` before their issue or PR prompt.

## Scheduler invariants

1. One PR equals one agent, one never-reused worktree, one branch and one file lease.
2. The scheduler creates worktrees only after all start gates are merged.
3. Agents do not communicate or share worktrees. The documents and issue tracker
   are their complete coordination channel.
4. Only the scheduler changes issue status or grants the Very High CI lease.
5. Within a PR, issues execute in listed order unless their dependency fields
   require a stricter order.
6. After merge: record receipts, close completed issues, delete the worktree and
   branch, fetch merged `main`, then schedule the next batch.
7. The backlog is immutable. Scope discoveries become comments and new issues.

## Worktree allocation

For execution run `<run-id>`:

```text
branch:   codex/pr-<NN>-<slug>-<run-id>
worktree: ../argot-wt-pr-<NN>-<run-id>
base:     latest fetched origin/main after prerequisite merges
```

The scheduler records branch, worktree, base SHA, owner, issue list and file
lease before starting the agent. A path used by any previous worktree is never
used again, even after deletion.

## Batch A — Baseline truth

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| A01 | PR-01 | CI-01–CI-05 | Distribution | Latest merged `main` | High |
| A02 | PR-02 | EV-01–EV-03, EV-05, EV-06, CL-01 | Evidence and claims | Latest merged `main` | Low |

- **Parallelism:** A01 and A02 start and merge independently.
- **Decision checkpoints:** EV-01 → DR-02; EV-05 → DR-10; EV-06 + CL-01 → DR-11.
- **Merge gate:** PR-01 and PR-02 both merged; both worktrees destroyed.
- **Exit condition:** Released install path is testable and all downstream agents
  have one evidence/claim contract.

## Batch B — Stable contracts and claim data

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| B01 | PR-03 | CLI-01–CLI-06, CLI-10/11, CI-06/07, MC-01/02 | Rust check contracts | PR-02 merged | Medium |
| B02 | PR-04 | EV-04, BM-01–BM-05 | Benchmark claims | PR-01 merged | Medium |

- **Parallelism:** B01 and B02 use disjoint leases and may merge independently.
- **Decision checkpoints:** CLI-01 → DR-01 → CLI-02; CLI-03/04 → DR-04/05;
  DR-05 + CL-01 → DR-08; BM-01–03 → DR-09 → BM-04/05. DR-14 must be
  resolved from EV-03 before CLI-05/06.
- **Merge gate:** PR-03 and PR-04 both merged; both worktrees destroyed.
- **Exit condition:** Human and machine check contracts are stable and every
  public metric has typed provenance.

## Batch C — Activation and retention evidence

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| C01 | PR-05 | CLI-07–CLI-09, AU-01–AU-04 | Rust audit activation | PR-03 merged | Medium |
| C02 | PR-06 | HK-01–HK-04, PL-01/02 | Lifecycle feasibility | PR-02 and PR-03 merged | High |
| C03 | PR-07 | BM-06–BM-09 | Benchmark claims | PR-03 and PR-04 merged; DR-03 resolved | **Very High** |

- **Parallelism:** C01, C02 and C03 may develop and review concurrently. Their
  file leases are disjoint. C03 alone holds the Very High CI lease.
- **Decision checkpoint:** after PR-06 and PR-07 merge, resolve DR-07 as
  ship/defer/reject. Do not start integration packaging earlier.
- **Merge gate:** PR-05, PR-06 and PR-07 merged; DR-07 resolved; all worktrees destroyed.
- **Exit condition:** Audit leads to recurring choices, lifecycle feasibility is
  measured and combined signal quality has a frozen verdict.

## Batch D — Shipped integrations and proof

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| D01 | PR-08 | PL-03–PL-06, PC-01/02, IN-01, SK-01–SK-04 | Integration packaging | PR-05/06/07 merged; DR-06/07 resolved | High |
| D02 | PR-09 | AS-01–AS-04 | Proof assets | PR-05/07 merged; DR-10 resolved | Medium |

- **Parallelism:** D01 and D02 start together and may merge independently.
- **Conditional scope:** when DR-07 is defer/reject, PL-03/04/05 remain blocked;
  D01 must not rewrite them or simulate a shipped lifecycle.
- **Operational checkpoint:** if DR-07 is ship, run REL-03 after PR-08 merges.
  A failed/deferred canary forbids automatic current-tense public copy.
- **Merge gate:** PR-08 and PR-09 merged; worktrees destroyed; REL-03 verdict
  recorded when applicable.
- **Exit condition:** Integration capability data exactly matches released
  behavior and proof inputs are reproducible.

## Batch E — Canonical journey documentation

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| E01 | PR-10 | DOC-01–DOC-08 | Documentation journeys | PR-08 merged | Medium |

- **Parallelism:** One docs owner; no other agent may touch docs navigation or content.
- **Merge gate:** PR-10 merged and worktree destroyed.
- **Exit condition:** The audit-to-habit and integration paths have stable routes
  and executable canonical instructions.

## Batch F — Canonical reference documentation

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| F01 | PR-11 | DOC-09–DOC-16 | Documentation reference | PR-04/07/08/10 merged | Medium |

- **Parallelism:** One reference-doc owner; landing and README worktrees do not yet exist.
- **Merge gate:** PR-11 merged and worktree destroyed.
- **Exit condition:** Configuration, trust, architecture, evidence, troubleshooting
  and generated agent-facing exports agree with shipped reality.

## Batch G — Public repositioning

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| G01 | PR-12 | LD-01–LD-12 | Landing product | PR-04/08/09/10/11 merged; DR-11/13 resolved; REL-03 passed if applicable | High |

- **Parallelism:** One landing owner; no landing QA or README worktree exists yet.
- **Merge gate:** PR-12 merged and worktree destroyed.
- **Exit condition:** The public site is behavior-led, audit-first, evidence-backed
  and uses only claims unlocked for the released product.

## Batch H — Public validation and README

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| H01 | PR-13 | LD-13–LD-16, AS-05 | Landing product | PR-12 merged | **Very High** |
| H02 | PR-14 | RD-01–RD-04 | README | PR-12 merged; DR-13 resolved | Low |

- **Parallelism:** H01 and H02 are file-disjoint and may merge independently.
  H01 exclusively holds the Very High CI lease.
- **Merge gate:** PR-13 and PR-14 merged; both worktrees destroyed.
- **Exit condition:** Landing route/accessibility/media gates pass and the README
  presents the same shipped journey without duplicating reference docs.

## Batch I — Release candidate

| Agent | PR | Issues | Exclusive owner | Start gate | CI |
| --- | --- | --- | --- | --- | --- |
| I01 | PR-15 | ON-01, QA-01/02, REL-01/02 | Release validation | PR-13/14 and every applicable earlier PR merged | **Very High** |

- **Parallelism:** None. I01 is the only active source owner and holds the Very High CI lease.
- **Failure rule:** An out-of-lease failure creates a new issue for the owning area;
  I01 never patches product/docs/landing/README opportunistically.
- **Merge gate:** PR-15 merged, release candidate approved and worktree destroyed.
- **Exit condition:** Clean-install journeys, version consistency, migration notes
  and repository-wide claims pass against one candidate SHA.

After release, execute REL-04 against the exact tag. It produces receipts, not a
retroactive release PR.

## Deferred lane

EV-07 has no reserved agent, worktree, branch or PR. It becomes schedulable only
after its evidence gate crosses. DR-12 likewise creates no implementation lane
unless its local-only history specification is explicitly approved.

## Dependency graph

```text
Batch A:  PR-01 ───────────────> PR-04 ───────┐
          PR-02 ──> PR-03 ─────> PR-07 ───────┼─> PR-08 ──> PR-10 ──> PR-11
                       ├───────> PR-05 ───────┤       │                  │
                       └───────> PR-06 ───────┘       │                  │
                           PR-05 + PR-07 ──> PR-09 ───┘                  │
                                                                         v
                                      PR-04/08/09/10/11 ─────────────> PR-12
                                                                         │
                                                            ┌────────────┴────────────┐
                                                            v                         v
                                                          PR-13                     PR-14
                                                            └────────────┬────────────┘
                                                                         v
                                                                       PR-15
```

No arrow may point to an open PR in actual execution. The target worktree is
created only after every source node on its incoming arrows is merged.

## Maximum safe concurrency

| Window | Maximum active PR agents | Reason |
| --- | ---: | --- |
| Batch A | 2 | Distribution and evidence are disjoint |
| Batch B | 2 | Rust contracts and benchmark data are disjoint |
| Batch C | 3 | Audit, lifecycle and benchmark harness have explicit leases |
| Batch D | 2 | Integration packaging and proof assets are disjoint |
| Batches E–G | 1 | Canonical docs and landing facts intentionally serialize |
| Batch H | 2 | Landing QA and README are disjoint |
| Batch I | 1 | Final candidate requires one immutable SHA |

Launching more agents than these limits creates coordination work without
increasing safe throughput.

## Scheduler handoff record

Before dispatching any agent, record:

```text
run id:
PR:
issue IDs:
owner:
base main SHA:
branch:
new worktree path:
leased paths:
forbidden paths:
CI class:
merged prerequisites:
decision gates:
```

The completion record contains local commands/results, CI URL/result, merge SHA,
closed/blocked/split issues, new follow-up issue URLs and worktree deletion proof.
