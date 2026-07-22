# Argot GitHub execution backlog

**Source:** [`docs/strategy/ARGOT_EXECUTION_PLAN.md`](../strategy/ARGOT_EXECUTION_PLAN.md)

**Canonical strategy:** [`docs/strategy/ARGOT_STRATEGY.md`](../strategy/ARGOT_STRATEGY.md)
**Prepared:** 2026-07-22

This is the normalized issue backlog for executing the master plan. It does not replace or amend the plan. Issue IDs are stable planning identifiers until real GitHub issue numbers are assigned. Decision issues are HITL and create no implementation PR. Implementation and research issues are AFK unless explicitly marked otherwise.

## Execution lock

This backlog becomes immutable when the final execution-program PR merges. From
that point forward, an execution agent may only complete, block or split an
issue in the GitHub tracker. It must never rewrite, merge, redefine or silently
expand an existing issue, and it must never edit this file. A split preserves
the original issue body, marks it blocked with a comment, and creates one or
more new issues. Newly discovered work always becomes a new issue.

Every issue has one exclusive owner below. “Owner” names an execution lane, not
a person; only one worktree may hold that lane at a time. The lane-to-filesystem
contract and worktree rules live in `AGENT_EXECUTION_RULES.md` and
`PR_PLAN.md`.

## Normalization decisions

### Canonical strategy-reference corrections

The master plan contains shorthand and several incorrect D-number mappings. It remains unchanged by request. Every issue below uses this canonical register:

| Canonical ID | Exact subject | Common incorrect use corrected here |
| --- | --- | --- |
| D1 | Behavioral invariant is the foundational belief | Not a standalone accept-time implementation decision |
| D2 | Audit installs; check-on-accept retains | Sometimes swapped with D3 |
| D3 | Build and market separate acquisition and retention engines | Sometimes described as the D2 operating-model sentence |
| D4 | Frame retention as awareness, not defect detection | Not specifically “audit should use memorable proof” |
| D5 | North Star is audit-to-habit conversion | Not the signal-quality or speed decision |
| D6 | Conviction on the foundation; options on the destination | Not the model-free-core boundary |
| D7 | Fully local individual core remains free, no account/payment | Correct in the master plan |
| D8 | Pursue onboarding that runs the check at the nearest acceptance lifecycle without a manual step | Not the no-telemetry decision |
| D9 | No future-specific work before its evidence gate is crossed | Not the portable-configuration principle |
| D10 | “Voice” is secondary brand/visual language, never the explanation | Correct in the master plan |
| D11 | Keep the four positioning layers separate | Not a generic future-work gate |
| D12 | No generative/opinion-forming model in the authoritative analytical core | Not the general honest-claims decision |
| D13 | Local-first, no default telemetry; enumerated default egress | Not the governance gate |
| D14 | Signal quality is existential; no default-gating detector above the defined noise threshold | Not the North Star decision |

Consequences for normalization:

- Accept-time lifecycle work cites D2, D4, D8 and D14.
- Audit acquisition work cites D2, D3 and D5.
- Public positioning cites D1, D4, D10 and D11.
- Privacy/network work cites D13; trusted-core/model wording cites D12.
- Future history, broader agents and organization-facing work cite D6 and D9.
- Embeddability and portable configuration are product principles, not mislabeled standing decisions.

### Issue shape and ownership rules

- Implementation issues are scoped to 30–90 minutes of focused work. Long-running benchmark/CI time is excluded from active effort but must be recorded in validation.
- Decision issues are resolved before their dependent implementation issue becomes `ready-for-agent`.
- Issues sharing a file live in one PR batch with one owner and execute serially.
- Public claims are changed once at their canonical owner, then consumed or linked elsewhere.
- No issue edits the strategy corpus or `ARGOT_EXECUTION_PLAN.md`.
- No issue edits this backlog or another issue after the execution lock.
- Gated future work creates no implementation issue until its decision gate passes.

### Owner vocabulary

| Owner | Exclusive responsibility |
| --- | --- |
| Decision owner | Human product, compatibility and release decisions |
| Evidence and claims | Evidence records, prototypes and the claim dictionary |
| Distribution | Action packaging, installers and distribution smoke |
| Rust check contracts | Check rendering/schema, machine output and MCP descriptions |
| Rust audit activation | Root/audit/init/unfitted activation flow |
| Benchmark claims | Benchmark data, manifest, harness and published results |
| Lifecycle feasibility | Pre-write hook and non-shipping Claude lifecycle prototype |
| Integration packaging | Released plugin, pre-commit, capability data and skills |
| Documentation journeys | Docs navigation, onboarding and integration journeys |
| Documentation reference | Configuration, trust, architecture and contributor references |
| Proof assets | Authored/wild fixtures, receipts and generated proof media |
| Landing product | Landing routes, product story, localization and landing QA |
| README | Root README only |
| Release validation | End-to-end, compatibility, canary and release verification |
| Deferred plugin evidence | One later evidence-gated non-Claude lifecycle investigation |

## Label vocabulary

Existing repository triage labels are preserved. The remaining labels are proposed GitHub labels for this program.

| Label | Meaning |
| --- | --- |
| `needs-triage` | Canonical repository status: not ready to execute |
| `ready-for-agent` | Canonical repository status: complete AFK specification |
| `ready-for-human` | Canonical repository status: HITL judgment required |
| `type:decision` | Human product/compatibility/default decision |
| `type:research` | Evidence collection or prototype; no product behavior promised |
| `type:implementation` | Product/repository behavior change |
| `type:docs` | Documentation or public copy |
| `type:qa` | Verification infrastructure or release validation |
| `priority:p0` | Blocks honest repositioning or a currently promoted integration |
| `priority:p1` | Blocks activation/retention |
| `priority:p2` | Foundation or quality improvement |
| `priority:p3` | Evidence-gated later option |
| `area:cli` | Rust CLI/check/report ownership |
| `area:audit` | Audit command/renderers |
| `area:benchmarks` | Benchmark harness/data/claims |
| `area:ci` | Action, workflows, distribution smoke |
| `area:plugin` | Claude hook/plugin/MCP |
| `area:skills` | Agent skills and their validation |
| `area:landing` | Astro homepage/routes/metadata/accessibility |
| `area:readme` | Root README |
| `area:docs` | User/contributor/security docs |
| `area:assets` | Reproducible screenshots, recordings and proof receipts |
| `area:release` | Compatibility, migration, canary and publish checks |

## Decision Required issues

These are deliberately separate from coding. Resolve each with a dated outcome, rationale, rejected alternatives and explicit implementation unlocks.

### DR-01 — Decide confidence-filter exit semantics

- **Goal:** Choose how `--min-confidence` affects display and process exit status.
- **Owner:** `Decision owner`
- **Why:** Current help says display-only while `gate_exit_code` receives filtered findings. D14 protects signal quality; portable/stable CLI semantics are a product principle.
- **Scope:** Decide between gate-on-all-unsuppressed findings with an explicit hidden-hit notice, rejecting incoherent combinations, or a documented alternative.
- **Files affected:** Decision record only; likely follow-up touches `crates/argot-engine/src/check/orchestrate.rs` and CLI docs.
- **Out of scope:** Threshold tuning, severity-default changes, or detector removal.
- **Dependencies:** None.
- **Acceptance criteria:** One option is selected; exit codes for warn/error/hidden/suppressed cases are tabulated; CLI-02 is unblocked.
- **Validation:** Review against current `CheckCmd` help, `gate_exit_code`, JSON/GitHub behavior and D14.
- **Estimated complexity:** 30 minutes.
- **Labels:** `ready-for-human`, `type:decision`, `priority:p0`, `area:cli`.

### DR-02 — Decide the automatic-brief exposure policy

- **Goal:** Fix which shipped rules/severities create an automatic brief and what counts as one interruption.
- **Owner:** `Decision owner`
- **Why:** Combined noise and lifecycle behavior cannot be measured without a frozen exposure policy. D2, D4, D8 and D14 govern it.
- **Scope:** Default release feature set, error/warn treatment, confidence presentation, deduplication window, clean behavior and setup-error behavior.
- **Files affected:** Decision record; later benchmark and plugin work consumes it.
- **Out of scope:** Changing detector thresholds, adding rules or making the brief blocking.
- **Dependencies:** EV-01.
- **Acceptance criteria:** A machine-testable policy table distinguishes finding, displayed hit, brief, and exit status; DR-03 and BM-06 are unblocked.
- **Validation:** Compare with `argot rules`, release features, default config, Action and hook semantics.
- **Estimated complexity:** 45 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p0`, `area:benchmarks`.

### DR-03 — Set combined quality and latency release gates

- **Goal:** Predeclare the thresholds that an automatic lifecycle must pass.
- **Owner:** `Decision owner`
- **Why:** D14 forbids allowing launch pressure to redefine acceptable noise after results are known.
- **Scope:** Findings/accepted change, briefs/accepted change, false/dismissed/uncertain classification, clean/noisy p95 latency, repeat-brief rate and failure budget.
- **Files affected:** Decision record and combined-evaluation protocol.
- **Out of scope:** Selecting corpora after seeing results or using the base foreign rate as the union threshold.
- **Dependencies:** DR-02, EV-02.
- **Acceptance criteria:** Numeric or explicitly qualitative pass/fail gates are approved before BM-09 runs; failure behavior is defined.
- **Validation:** Cross-check against P0-2, P1-2, D14 and available accepted-history data.
- **Estimated complexity:** 45 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p0`, `area:benchmarks`.

### DR-04 — Approve the check JSON v1 compatibility contract

- **Goal:** Decide required fields and additive/breaking-change policy for `check --format json`.
- **Owner:** `Decision owner`
- **Why:** A version field is an external compatibility promise, not merely serialization work.
- **Scope:** Top-level versioning, required fields, unknown-field tolerance, deprecation and consumer support window.
- **Files affected:** Decision record; follow-up `crates/argot-engine/src/output.rs`, schema and docs.
- **Out of scope:** Secondary command JSON contracts or finding-semantic redesign.
- **Dependencies:** DR-01.
- **Acceptance criteria:** Schema rules are approved and CLI-03/04 are unblocked.
- **Validation:** Compare with audit JSON v1, current fixtures, Action/skill consumers and SARIF.
- **Estimated complexity:** 30 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p2`, `area:cli`.

### DR-05 — Classify secondary JSON outputs

- **Goal:** Decide which status/list/inspect/rules/conventions/suggest/voice-diff JSON outputs are public contracts.
- **Owner:** `Decision owner`
- **Why:** Calling ad hoc output stable creates accidental API commitments.
- **Scope:** Classify each output as versioned public, best-effort/internal or deprecated; create follow-up issues only for public contracts.
- **Files affected:** Decision record and later command-reference update.
- **Out of scope:** Implementing every schema or renaming fields.
- **Dependencies:** DR-04, CLI-04.
- **Acceptance criteria:** Every machine-readable command has a rationale-backed class and owner.
- **Validation:** Search repository consumers and sample every output.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p2`, `area:cli`.

### DR-06 — Decide pre-commit default behavior

- **Goal:** Resolve whether the shipped hook remains blocking or becomes advisory by default.
- **Owner:** `Decision owner`
- **Why:** Current behavior blocks on error findings while docs say informational. This is a user-visible default and migration decision; D14 and the “surface, don’t enforce” contract constrain it but do not silently choose it.
- **Scope:** Finding exit behavior, setup-error behavior, explicit gating recipe, migration and naming.
- **Files affected:** Decision record; later `.pre-commit-hooks.yaml`, wrapper/CLI, tests and CI docs.
- **Out of scope:** General `argot check` exit changes or Action defaults.
- **Dependencies:** DR-01.
- **Acceptance criteria:** Default and opt-in gate are unambiguous; PC-01/02 are unblocked.
- **Validation:** Walk clean, error-hit, warn-hit, unfitted and command-error cases.
- **Estimated complexity:** 30 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p0`, `area:plugin`.

### DR-07 — Decide whether the Claude end-of-turn prototype ships

- **Goal:** Convert prototype and combined-quality evidence into a ship/reject/defer decision.
- **Owner:** `Decision owner`
- **Why:** A reachable Stop event is not proof that it is a trustworthy acceptance proxy. D2, D8 and D14 require evidence first.
- **Scope:** Trigger semantics, dedupe, latency, user interrupt, recursion, failure behavior, opt-out and allowed claim.
- **Files affected:** Decision record; later plugin manifest/hook implementation and release notes.
- **Out of scope:** Other agents or universal accept-time language.
- **Dependencies:** BM-09, PL-02, DR-14.
- **Acceptance criteria:** Outcome is ship/reject/defer; the exact allowed public sentence and implementation scope are recorded.
- **Validation:** Compare prototype receipts with DR-03 gates and canonical current-reality boundaries.
- **Estimated complexity:** 45 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p0`, `area:plugin`.

### DR-08 — Decide the future of `voice-diff` and the Action score

- **Goal:** Choose a compatibility-safe replacement for “100% in-voice” and the aggregate voice score.
- **Owner:** `Decision owner`
- **Why:** Absence of configured findings is not repository conformity; changing command/card/badge semantics can break consumers.
- **Scope:** Preserve/deprecate command name, human/card wording, badge meaning, output-field compatibility and migration window.
- **Files affected:** Decision record; later `voice_diff.rs`, `action.yml`, docs and release notes.
- **Out of scope:** New correctness score or detector changes.
- **Dependencies:** DR-05, CL-01.
- **Acceptance criteria:** One metric/writing contract is approved with compatibility treatment; CI-06 is unblocked.
- **Validation:** Inspect Action, badge and documented consumers; enforce clean-run claim boundary.
- **Estimated complexity:** 45 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p0`, `area:ci`.

### DR-09 — Select canonical benchmark lineages

- **Goal:** Resolve which foreign, architecture and integrity artifacts are current public sources.
- **Owner:** `Decision owner`
- **Why:** Public numbers conflict; selecting one is an evidence/provenance decision, not copy editing.
- **Scope:** 595/605 vs 604/618, 244/252 vs 264/272, 144/153 vs 155/164 and their denominators/qualifiers.
- **Files affected:** Decision record and claim-manifest source mapping.
- **Out of scope:** Rerunning benchmarks solely to obtain a preferred result.
- **Dependencies:** BM-01, BM-02, BM-03.
- **Acceptance criteria:** Each disputed claim has canonical artifact, revision, scope, allowed wording and superseded values.
- **Validation:** Recompute percentages and trace every numerator/denominator to raw or dated evidence.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p0`, `area:benchmarks`.

### DR-10 — Decide the Caught-in-the-Wild claim disposition

- **Goal:** Choose whether to retain a verified corpus count/cases or reduce the page to the evidence available.
- **Owner:** `Decision owner`
- **Why:** “33 repositories” and five stories lack a complete committed evidence bundle; D11 requires memorable proof to remain an independently evidenced positioning layer.
- **Scope:** Retained count, case set, reconstruction labels, upstream-link requirements and hash presentation.
- **Files affected:** Decision record; later case data/page/receipts.
- **Out of scope:** Inventing cases or treating authored fixtures as wild catches.
- **Dependencies:** EV-05.
- **Acceptance criteria:** Each retained public fact has a required receipt, or unsupported material is explicitly removed.
- **Validation:** Review evidence inventory, licensing/privacy constraints and reproduction results.
- **Estimated complexity:** 30 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p0`, `area:assets`.

### DR-11 — Decide whether the launch film is retained

- **Goal:** Choose retain/update/demote/remove based on claim, accessibility and provenance cost.
- **Owner:** `Decision owner`
- **Why:** The remote film currently delays audit proof and lacks committed transcript/captions/source/checksum.
- **Scope:** Launch-path position, required transcript/captions/source provenance and acceptable claims.
- **Files affected:** Decision record; later landing/asset issue.
- **Out of scope:** New brand strategy or unrelated video production.
- **Dependencies:** EV-06, CL-01.
- **Acceptance criteria:** One disposition and exact implementation checklist are recorded.
- **Validation:** Review remote media, poster claims, mobile/a11y behavior and D10/D11 boundaries.
- **Estimated complexity:** 30 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p2`, `area:assets`.

### DR-12 — Decide whether durable local finding history is justified

- **Goal:** Pass, defer or reject a bounded local-history product specification.
- **Owner:** `Decision owner`
- **Why:** P2-1 is explicitly evidence-gated; D6/D9 forbid future-specific work without evidence and D13 forbids default telemetry.
- **Scope:** Present-day user value, storage, schema, retention, delete/export, opt-in/out, privacy and relationship to `last-check.json`.
- **Files affected:** Decision record only; implementation issues are created only after approval.
- **Out of scope:** Cloud/team aggregation, dashboards, telemetry or a launch-blocker designation.
- **Dependencies:** BM-09 and Claude pilot evidence if it ships.
- **Acceptance criteria:** Outcome is reject/defer or an approved minimal specification with measurable local user value.
- **Validation:** Threat-model review, storage estimate and proof that accepted-history replay cannot meet the same need more cheaply.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p3`, `area:cli`.

### DR-13 — Approve release and claim-unlock boundaries

- **Goal:** Define which fixes ship as immediate truth correction, foundation release, integration canary and public repositioning.
- **Owner:** `Decision owner`
- **Why:** Public copy may become more honest immediately, but behavior-dependent claims must wait for released evidence.
- **Scope:** Minimum dependency per claim, compatibility changes, canary failure path and versioning boundaries.
- **Files affected:** Decision record and release checklist.
- **Out of scope:** Reopening positioning or delaying known factual corrections.
- **Dependencies:** DR-04, DR-06, DR-08 and preliminary DR-07 status.
- **Acceptance criteria:** Every planned PR has a release phase and no public claim has a circular or unshipped dependency.
- **Validation:** Dry-run against PR_PLAN and the canonical claim ledger.
- **Estimated complexity:** 45 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p0`, `area:release`.

### DR-14 — Select the human briefing layout

- **Goal:** Choose the zero/one/many finding hierarchy from tested prototypes.
- **Owner:** `Decision owner`
- **Why:** P1-2 is an evidence-required UX decision; implementation should not bake in an unreviewed hierarchy.
- **Scope:** First lines, severity/rule order, evidence placement, clean wording, truncation, mute/inspect affordance and line budget.
- **Files affected:** Decision record; later `check/render.rs` snapshots.
- **Out of scope:** Detector scoring, machine formats or lifecycle implementation.
- **Dependencies:** EV-03, DR-02.
- **Acceptance criteria:** Selected prototype and rejected alternatives are recorded; CLI-05/06 are unblocked.
- **Validation:** Structured comprehension review with fresh and experienced users or documented proxy limitations.
- **Estimated complexity:** 45 minutes.
- **Labels:** `needs-triage`, `type:decision`, `priority:p1`, `area:cli`.

## Evidence and program-contract issues

### EV-01 — Inventory released integration behavior

- **Goal:** Produce the tested capability matrix that distinguishes automatic, passive, invoked and user-wired surfaces.
- **Owner:** `Evidence and claims`
- **Why:** P0-1 and D8 require knowing which lifecycle is genuinely reachable before implementation or claims.
- **Scope:** Claude plugin/pre-write hook, six skills, MCP, pre-commit, Action, manual CLI/review, and current official lifecycle evidence for named other agents.
- **Files affected:** New `docs/research/evidence/integration-capability-matrix.md` and receipts beside it.
- **Out of scope:** Shipping hooks, promising support or testing “70+” hosts.
- **Dependencies:** None.
- **Acceptance criteria:** Every surface lists trigger, coverage, prerequisites, automation class, failure/blocking behavior, tested version/date and safe wording.
- **Validation:** Manual smoke where runnable; cite repository source and current official vendor source.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `ready-for-agent`, `type:research`, `priority:p0`, `area:plugin`.

### EV-02 — Write the combined-brief evaluation protocol

- **Goal:** Convert DR-02 into a reproducible benchmark protocol and gate template for DR-03.
- **Owner:** `Evidence and claims`
- **Why:** P0-2 cannot be closed with per-detector numbers.
- **Scope:** Accepted-change unit, sampling, adjudication, denominators, latency capture and raw-record format.
- **Files affected:** New `docs/research/evidence/accept-brief-protocol.md`; benchmark fixture manifest path reserved for BM-06.
- **Out of scope:** Implementing the harness or running the full evaluation.
- **Dependencies:** DR-02.
- **Acceptance criteria:** Another agent can implement the harness without choosing metrics, sampling or labels.
- **Validation:** Dry-run protocol on three accepted changes and reconcile counts by hand.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:research`, `priority:p0`, `area:benchmarks`.

### EV-03 — Produce zero/one/many briefing prototypes

- **Goal:** Create fixed text prototypes for the accept-time human brief.
- **Owner:** `Evidence and claims`
- **Why:** P1-2 requires evidence before editing `render.rs`.
- **Scope:** Clean, one actionable finding, many mixed findings, hidden/suppressed findings, stale fit and setup error.
- **Files affected:** New `docs/research/evidence/accept-brief-prototypes.md` and plain-text fixtures outside production snapshots.
- **Out of scope:** Rust changes or deciding the winning layout.
- **Dependencies:** DR-02.
- **Acceptance criteria:** Each prototype fits a declared terminal width/line budget and preserves evidence/human-last-word wording.
- **Validation:** Side-by-side comprehension script ready for DR-14.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:research`, `priority:p1`, `area:cli`.

### EV-04 — Measure ordinary-repository cold and warm audit timing

- **Goal:** Record repeatable audit timing for small and medium pinned repositories.
- **Owner:** `Benchmark claims`
- **Why:** Current “sixty seconds/two minutes” wording lacks a stable scope.
- **Scope:** Cold model fetch separated from analysis; warm and offline cases; hardware/network metadata.
- **Files affected:** New dated evidence record and raw timing receipts under `docs/research/evidence/`.
- **Out of scope:** Large repositories or performance optimization.
- **Dependencies:** CI-01 so the released install path is valid.
- **Acceptance criteria:** At least two pinned repositories have repeated wall-time, phase-time and memory records.
- **Validation:** Two runs per case; totals reconcile with CLI phases.
- **Estimated complexity:** 45–75 minutes active.
- **Labels:** `needs-triage`, `type:research`, `priority:p1`, `area:benchmarks`.

### EV-05 — Audit Caught-in-the-Wild evidence

- **Goal:** Locate and verify receipts behind the five cases and 33-repository count.
- **Owner:** `Evidence and claims`
- **Why:** D11’s memorable-proof layer must be inspectable without overstating evidence.
- **Scope:** Repo/commit, command/range, actual finding JSON/hash, upstream URL, date, adjudication and reconstruction status.
- **Files affected:** New evidence inventory under `docs/research/evidence/`; no public page changes.
- **Out of scope:** Creating replacement stories or changing public counts.
- **Dependencies:** None.
- **Acceptance criteria:** Every existing claim is verified, unverifiable or blocked by a named licensing/privacy constraint.
- **Validation:** Re-run accessible cases and link-check upstream sources.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `ready-for-agent`, `type:research`, `priority:p0`, `area:assets`.

### EV-06 — Inventory launch-film claims and assets

- **Goal:** Establish what the remote film says and what source/accessibility artifacts exist.
- **Owner:** `Evidence and claims`
- **Why:** DR-11 needs facts before choosing retain/update/remove.
- **Scope:** Transcript, captions, poster/film claims, remote URL/checksum/source ownership and homepage interaction.
- **Files affected:** New `docs/research/evidence/launch-film-inventory.md`.
- **Out of scope:** Editing video, landing code or strategy.
- **Dependencies:** None.
- **Acceptance criteria:** Complete claim transcript and asset/provenance/accessibility inventory exists.
- **Validation:** Compare rendered film/poster with repository references.
- **Estimated complexity:** 45 minutes.
- **Labels:** `ready-for-agent`, `type:research`, `priority:p2`, `area:assets`.

### CL-01 — Publish the maintained public claim dictionary

- **Goal:** Create one operational claim source from the execution-plan ledger and canonical D-register.
- **Owner:** `Evidence and claims`
- **Why:** P0-3 cannot be fixed consistently while every surface invents qualifiers.
- **Scope:** Allowed/forbidden wording, evidence owner, current/future status, numeric-manifest key and compatibility exceptions.
- **Files affected:** New `docs/execution/PUBLIC_CLAIMS.md`.
- **Out of scope:** Editing any consumer surface or copying the full strategy.
- **Dependencies:** EV-01; DR-09 for disputed numeric entries, which may remain explicitly unresolved until then.
- **Acceptance criteria:** Every master-plan claim-ledger row is represented and all D references use the canonical mapping above.
- **Validation:** Cross-check strategy §10/§19 and current reality; repository phrase search produces a consumer worklist.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:docs`.

## Rust check and CLI issues

### CLI-01 — Add confidence/exit regression coverage

- **Goal:** Capture current `--min-confidence` behavior before changing it.
- **Owner:** `Rust check contracts`
- **Why:** DR-01 is a compatibility decision and needs an executable baseline.
- **Scope:** Warn/error, `--error-on-warnings`, three tiers, suppressed hits and human/JSON/GitHub formats.
- **Files affected:** Focused tests beside `crates/argot-engine/src/check/orchestrate.rs` or existing check integration tests.
- **Out of scope:** Production behavior or help text.
- **Dependencies:** None.
- **Acceptance criteria:** Tests demonstrate the current mismatch and can be updated to the chosen contract.
- **Validation:** Targeted cargo test passes with current expected behavior.
- **Estimated complexity:** 45 minutes.
- **Labels:** `ready-for-agent`, `type:qa`, `priority:p0`, `area:cli`.

### CLI-02 — Implement decided confidence/exit behavior

- **Goal:** Make filtering and exit status match DR-01.
- **Owner:** `Rust check contracts`
- **Why:** Current implementation and public contract disagree.
- **Scope:** Gate input selection and any required hidden-hit diagnostic.
- **Files affected:** `crates/argot-engine/src/check/orchestrate.rs`; CLI-01 tests.
- **Out of scope:** Confidence calibration, severity defaults or documentation pages.
- **Dependencies:** DR-01, CLI-01.
- **Acceptance criteria:** All DR-01 cases pass and changing display tier cannot silently weaken the selected gate semantics.
- **Validation:** Targeted tests plus human/JSON/GitHub manual samples.
- **Estimated complexity:** 45–75 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:cli`.

### CLI-03 — Add `schema_version` to check JSON

- **Goal:** Emit the approved JSON v1 identifier without changing finding semantics.
- **Owner:** `Rust check contracts`
- **Why:** P2-2 requires an explicit compatibility boundary.
- **Scope:** Top-level report serialization and clean/finding golden fixtures.
- **Files affected:** `crates/argot-engine/src/output.rs`, `crates/argot-core/tests/check_format.rs` and fixtures.
- **Out of scope:** Publishing the schema document or secondary outputs.
- **Dependencies:** DR-04, CLI-02.
- **Acceptance criteria:** Every check JSON report contains the approved version and existing fields retain their meaning.
- **Validation:** Serialization/golden tests.
- **Estimated complexity:** 30–60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p2`, `area:cli`.

### CLI-04 — Publish and validate check JSON Schema v1

- **Goal:** Provide a consumer-validatable schema and compatibility fixture.
- **Owner:** `Rust check contracts`
- **Why:** A version field alone does not make the machine contract usable.
- **Scope:** Schema artifact, clean/finding validation and unknown-field consumer fixture.
- **Files affected:** New schema under a stable docs/schema path, test fixture and command-reference link placeholder.
- **Out of scope:** Secondary command schemas.
- **Dependencies:** CLI-03.
- **Acceptance criteria:** Real clean/finding output validates and the compatibility policy is embedded or linked.
- **Validation:** JSON Schema test in Rust or repository tooling.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p2`, `area:cli`.

### CLI-05 — Implement clean and single-finding briefing states

- **Goal:** Apply the DR-14 layout to clean and one-finding output.
- **Owner:** `Rust check contracts`
- **Why:** These are the dominant accept-time states and must be concise without overclaiming.
- **Scope:** Banner, scan-bounded clean wording, one finding’s severity/rule/evidence/action and no-color behavior.
- **Files affected:** `crates/argot-engine/src/check/render.rs` and dedicated golden fixtures.
- **Out of scope:** Multi-finding grouping, machine output or lifecycle hooks.
- **Dependencies:** DR-14, CLI-02.
- **Acceptance criteria:** Selected prototypes match at declared widths; “style linter” and “looks clean” are absent.
- **Validation:** Golden tests for TTY/no-color and narrow terminal.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:cli`.

### CLI-06 — Implement multi-finding briefing hierarchy

- **Goal:** Apply the selected priority/group/truncation behavior to mixed findings.
- **Owner:** `Rust check contracts`
- **Why:** Noisy-looking output can destroy trust even when findings are valid.
- **Scope:** Ordering, counts, file grouping, truncation/verbose hint and suppressed/hidden notes.
- **Files affected:** `crates/argot-engine/src/check/render.rs` and multi-finding fixtures.
- **Out of scope:** Clean/single state or detector ordering changes outside presentation.
- **Dependencies:** CLI-05.
- **Acceptance criteria:** Mixed error/warn/rule cases match DR-14 and preserve hashes, spans and evidence.
- **Validation:** Golden tests at normal/narrow widths and verbose mode.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:cli`.

### CLI-07 — Single-source audit-first root help

- **Goal:** Remove drift between clap and the custom no-arg banner and make audit the first-run command.
- **Owner:** `Rust audit activation`
- **Why:** D2/D3/D5 make audit the acquisition front door; D10 forbids voice-led explanation.
- **Scope:** Command registry, root/no-arg tagline, ordering and complete command list.
- **Files affected:** `crates/argot-cli/src/main.rs` and root-help snapshots.
- **Out of scope:** Subcommand behavior or command renames.
- **Dependencies:** CL-01.
- **Acceptance criteria:** Root and no-arg help agree, include all public commands and lead with current-reality audit wording.
- **Validation:** Snapshot both entry paths and compare command IDs programmatically.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:cli`.

### CLI-08 — Improve the unfitted check error

- **Goal:** Offer audit for immediate proof and init for recurring use without mutating the repo.
- **Owner:** `Rust audit activation`
- **Why:** P1-1 setup friction currently produces a dead end.
- **Scope:** Missing baseline/config error only; preserve old/malformed artifact specificity.
- **Files affected:** `crates/argot-rules-voice/src/load.rs` and focused tests.
- **Out of scope:** Automatically running commands or changing exit code 2.
- **Dependencies:** CLI-07.
- **Acceptance criteria:** Cold error has two concise, distinct next actions and machine stderr remains predictable.
- **Validation:** Missing/old/malformed/machine-format cases.
- **Estimated complexity:** 30–45 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:cli`.

### CLI-09 — Add integration guidance to successful init

- **Goal:** End setup with a manual smoke check and a real recurring-path pointer.
- **Owner:** `Rust audit activation`
- **Why:** D2/D8 require the audit-to-habit handoff; current init ends at manual check.
- **Scope:** Ready, Ready-with-notes and Not-recommended success/next-action text.
- **Files affected:** `crates/argot-cli/src/main.rs` init rendering and snapshots.
- **Out of scope:** Installing integrations or changing fit’s artifact-only behavior.
- **Dependencies:** IN-01, CL-01; PL-05 only if its lifecycle is described as automatic.
- **Acceptance criteria:** No unavailable integration is recommended; health caveats remain prominent.
- **Validation:** Snapshot all verdict/offline states and link/command check.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:cli`.

### CLI-10 — Reframe SARIF and GitHub rule descriptions

- **Goal:** Remove “out of voice” explanatory copy while preserving identifiers and schemas.
- **Owner:** `Rust check contracts`
- **Why:** D10 applies to machine-integrated human descriptions too.
- **Scope:** SARIF rule descriptions and GitHub annotation titles/messages only.
- **Files affected:** `crates/argot-engine/src/output.rs` and format fixtures.
- **Out of scope:** Check JSON structure or Action summary/card.
- **Dependencies:** CL-01, CLI-03.
- **Acceptance criteria:** Stable IDs/properties remain; descriptions use repository-grounded evidence language.
- **Validation:** SARIF/annotation snapshots and schema validation.
- **Estimated complexity:** 30–45 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:cli`.

### CLI-11 — Add public CLI wording regression checks

- **Goal:** Prevent root/subcommand/help/render wording from drifting back to forbidden explanations.
- **Owner:** `Rust check contracts`
- **Why:** P0-3 spans Rust strings and currently lacks a guard.
- **Scope:** Root and all public subcommand help snapshots plus an allowlisted phrase test for user-visible Rust output.
- **Files affected:** CLI integration tests and fixtures only.
- **Out of scope:** Banning internal compatibility symbols or research terminology.
- **Dependencies:** CLI-05–10, AU-04.
- **Acceptance criteria:** Approved help/output is snapshotted and forbidden phrases fail outside explicit allowlist paths.
- **Validation:** `cargo test` targeted suite and `just verify`.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:cli`.

## Audit CLI issues

### AU-01 — Add audit net-window and attribution boundaries

- **Goal:** State audit’s base-to-head net-diff and marker-attribution method consistently.
- **Owner:** `Rust audit activation`
- **Why:** D11’s proof layer must be credible; current wording can imply commit replay or authorship inference.
- **Scope:** Help plus terminal/Markdown/HTML method note; additive JSON method metadata only if compatible with audit v1.
- **Files affected:** `crates/argot-cli/src/main.rs`, `crates/argot-cli/src/audit/{term,markdown,html,report}.rs`.
- **Out of scope:** Audit algorithm or attribution logic.
- **Dependencies:** CL-01.
- **Acceptance criteria:** All human renderers say surviving base-to-head patterns and marker-based floor/census limits.
- **Validation:** Cross-renderer text assertions.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:audit`.

### AU-02 — Reframe audit titles, empty state and share caption

- **Goal:** Replace voice/style/“AI snuck in” language with one repository-grounded proof story.
- **Owner:** `Rust audit activation`
- **Why:** D10 and D11 require brand and proof layers not to masquerade as the product explanation.
- **Scope:** Titles, empty state, share caption and report footer only.
- **Files affected:** `crates/argot-cli/src/audit/{term,markdown,html,report}.rs`.
- **Out of scope:** Method caveats or next-action CTA.
- **Dependencies:** AU-01, CL-01.
- **Acceptance criteria:** Empty state is scan-bounded; attribution shorthand remains qualified; share copy fits current card width.
- **Validation:** Renderer/caption snapshots.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:audit`.

### AU-03 — Add audit-to-habit next actions

- **Goal:** Guide from audit to init and the tested integration chooser.
- **Owner:** `Rust audit activation`
- **Why:** D2/D5 make audit-to-habit conversion the operating funnel.
- **Scope:** Terminal/Markdown/HTML CTA; additive structured next action only if audit JSON policy allows it.
- **Files affected:** `crates/argot-cli/src/audit/{term,markdown,html,report}.rs`.
- **Out of scope:** Auto-installation or unshipped lifecycle claims.
- **Dependencies:** AU-02, IN-01, CLI-09.
- **Acceptance criteria:** Commands/links exist, work offline where relevant and label automation accurately.
- **Validation:** Renderer snapshots and manual audit → init → chooser walkthrough.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:audit`.

### AU-04 — Add audit contract regression fixtures

- **Goal:** Lock cross-renderer boundaries and next actions after the rewrite.
- **Owner:** `Rust audit activation`
- **Why:** Terminal, Markdown, HTML, JSON and caption currently drift independently.
- **Scope:** Findings, empty, marker attribution, transient-added-then-removed and CTA snapshots.
- **Files affected:** Audit module tests/fixtures only.
- **Out of scope:** New production copy.
- **Dependencies:** AU-01–03.
- **Acceptance criteria:** All renderers share method/claim/CTA contract and a transient issue absent from net diff is tested.
- **Validation:** Targeted audit tests.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:audit`.

## CI, hooks and integration issues

### CI-01 — Fix Action archive resolution

- **Goal:** Download the archive format cargo-dist actually publishes.
- **Owner:** `Distribution`
- **Why:** The current `.tar.xz`/`.tar.gz` mismatch makes the promoted CI path unreliable.
- **Scope:** Unix artifact URL/name resolution and extraction; retain checksum verification.
- **Files affected:** `action.yml`, with `dist-workspace.toml` read as the contract.
- **Out of scope:** Action wording, new targets or distribution-format changes.
- **Dependencies:** None.
- **Acceptance criteria:** Current Linux/macOS release URLs resolve and install the requested Argot version.
- **Validation:** `cargo dist plan` comparison and tagged-release smoke.
- **Estimated complexity:** 30–45 minutes.
- **Labels:** `ready-for-agent`, `type:implementation`, `priority:p0`, `area:ci`, `bug`.

### CI-02 — Add Linux Action install and finding smoke

- **Goal:** Exercise Linux archive selection, installation, clean result and finding result.
- **Owner:** `Distribution`
- **Why:** Source agreement alone does not prove the composite Action works.
- **Scope:** Linux x64 hosted runner and an available arm64 path/emulation note.
- **Files affected:** New/updated `.github/workflows/` smoke workflow and minimal fixture.
- **Out of scope:** macOS/Windows or release publishing.
- **Dependencies:** CI-01.
- **Acceptance criteria:** Installed version matches input; default remains non-blocking; annotations/summary are produced.
- **Validation:** Workflow run receipt linked in issue.
- **Estimated complexity:** 60–90 minutes active.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:ci`.

### CI-03 — Add macOS Action install smoke

- **Goal:** Validate Intel/Arm archive selection and install on available macOS runners.
- **Owner:** `Distribution`
- **Why:** Both macOS targets are publicly distributed.
- **Scope:** Target mapping, extraction and `argot --version` for x64/arm64 as runner support permits.
- **Files affected:** Same Action smoke workflow owned serially after CI-02.
- **Out of scope:** Product checks beyond a minimal clean invocation.
- **Dependencies:** CI-02.
- **Acceptance criteria:** Both targets are tested or an unavailable runner is explicitly documented rather than claimed.
- **Validation:** Workflow receipt.
- **Estimated complexity:** 45–75 minutes active.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:ci`.

### CI-04 — Add Windows installer smoke

- **Goal:** Validate Windows x64 installer, dynamic UCRT wording and `argot --version`.
- **Owner:** `Distribution`
- **Why:** The platform claim must be tested and must not imply a universally static binary.
- **Scope:** PowerShell installer/target and uninstall ownership smoke.
- **Files affected:** Same smoke workflow/fixture; no public docs.
- **Out of scope:** New Windows targets.
- **Dependencies:** CI-02.
- **Acceptance criteria:** Clean Windows runner installs/runs/uninstalls the tagged binary.
- **Validation:** Workflow receipt and artifact log.
- **Estimated complexity:** 60–90 minutes active.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:ci`.

### CI-05 — Test checksum and missing-asset failures

- **Goal:** Prove Action installation fails safely and intelligibly on corrupt/missing assets.
- **Owner:** `Distribution`
- **Why:** Release reliability includes failure behavior, not only happy paths.
- **Scope:** Fixture/mocked archive checksum mismatch and missing asset response.
- **Files affected:** Action smoke fixtures/workflow.
- **Out of scope:** General supply-chain redesign.
- **Dependencies:** CI-02.
- **Acceptance criteria:** Both failure modes stop installation with actionable, secret-free output.
- **Validation:** Negative workflow/job tests.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p1`, `area:ci`.

### CI-06 — Implement the decided Action findings summary

- **Goal:** Replace conformance/voice-score language according to DR-08 without breaking approved inputs.
- **Owner:** `Rust check contracts`
- **Why:** A zero-hit change cannot be presented as full repository conformity.
- **Scope:** Action name/description, job summary, sticky comment/card/badge wording and any approved compatibility shim.
- **Files affected:** `action.yml`; `crates/argot-cli/src/voice_diff.rs` only if DR-08 requires it.
- **Out of scope:** Detector scoring or Action install behavior.
- **Dependencies:** DR-08, CI-02–05, CL-01.
- **Acceptance criteria:** Clean, warn and error states use the approved observed-findings contract; default is described as non-blocking.
- **Validation:** Action snapshots/fixtures for all states.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:ci`.

### CI-07 — Add Action output regression snapshots

- **Goal:** Prevent summary/comment/badge claims and markers from drifting.
- **Owner:** `Rust check contracts`
- **Why:** Action output is a public product surface and expensive to validate manually.
- **Scope:** Clean, warn, error, setup failure and optional-gate snapshots.
- **Files affected:** Action test fixtures/snapshot tooling only.
- **Out of scope:** New Action behavior.
- **Dependencies:** CI-06.
- **Acceptance criteria:** All major states are stable and current claim phrases are asserted.
- **Validation:** Local fixture test plus one hosted workflow run.
- **Estimated complexity:** 45–75 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:ci`.

### HK-01 — Honor rule enablement and severity in the pre-write hook

- **Goal:** Prevent a disabled foreign-import rule from prompting through the hook.
- **Owner:** `Lifecycle feasibility`
- **Why:** Portable, user-owned configuration is a foundation principle.
- **Scope:** Rule/group off/severity resolution for the pre-write assessment.
- **Files affected:** `crates/argot-cli/src/hook.rs` and focused fixtures.
- **Out of scope:** Path scopes, mutes/migrations or new lifecycle events.
- **Dependencies:** None.
- **Acceptance criteria:** Rule/group off produces no ask; supported severity behavior is documented/tested; errors remain non-blocking.
- **Validation:** Hook/CLI parity fixture.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `ready-for-agent`, `type:implementation`, `priority:p1`, `area:plugin`.

### HK-02 — Honor hook path scopes and exclusions

- **Goal:** Apply committed path scope/exclusion intent before a pre-write ask.
- **Owner:** `Lifecycle feasibility`
- **Why:** A repo should not be interrupted for paths the full check intentionally excludes.
- **Scope:** Path classification available at pre-write time, including local config precedence if applicable.
- **Files affected:** `crates/argot-cli/src/hook.rs`, config/path helpers and fixtures.
- **Out of scope:** Hash mutes or migrations.
- **Dependencies:** HK-01.
- **Acceptance criteria:** In-scope prompts; excluded/scoped-out paths do not; malformed config safely no-ops with diagnostic policy.
- **Validation:** Hook/CLI path matrix.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:plugin`.

### HK-03 — Define and implement migration/suppression parity subset

- **Goal:** Apply only suppression/migration semantics that map honestly to pre-write content.
- **Owner:** `Lifecycle feasibility`
- **Why:** Diff-hash mutes may not map to not-yet-written code; faking parity would be misleading.
- **Scope:** Declared migrations and the approved applicable mute/suppression subset, with explicit unsupported cases.
- **Files affected:** `crates/argot-cli/src/hook.rs` and fixtures.
- **Out of scope:** Inventing hashes or broadening suppression behavior.
- **Dependencies:** HK-02.
- **Acceptance criteria:** Declared replacement does not prompt as foreign; unsupported suppression types are explicitly documented/tested.
- **Validation:** Migration and suppression matrix against full-check intent.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:plugin`.

### HK-04 — Add timeout and fail-open hook contract tests

- **Goal:** Lock the current ask-only, never-block, unfitted/error behavior.
- **Owner:** `Lifecycle feasibility`
- **Why:** Hook failures must not interrupt coding or recurse.
- **Scope:** Manifest timeout, unfitted repo, bad input/config, unsupported file and internal error.
- **Files affected:** `hooks/hooks.json`, hook tests/fixtures.
- **Out of scope:** End-of-turn lifecycle.
- **Dependencies:** HK-03.
- **Acceptance criteria:** Manifest has explicit timeout; every failure exits successfully/no-ops under the approved diagnostic policy.
- **Validation:** JSON manifest parse and hook fixture suite.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p1`, `area:plugin`.

### PL-01 — Build the Claude Stop-event prototype harness

- **Goal:** Capture Stop/end-of-turn inputs and invoke a full changeset check in a disposable prototype.
- **Owner:** `Lifecycle feasibility`
- **Why:** D8 requires the nearest reachable lifecycle, but event semantics must be measured.
- **Scope:** Test-only/prototype hook configuration, event capture and full CLI invocation.
- **Files affected:** Prototype/evidence fixtures outside shipped manifest; `docs/research/evidence/` receipt.
- **Out of scope:** Shipping, dedupe or public claims.
- **Dependencies:** EV-01, DR-02, CLI-05/06.
- **Acceptance criteria:** Clean/noisy/unfitted turns can be invoked reproducibly and event payloads are recorded.
- **Validation:** Pinned Claude version manual run.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:research`, `priority:p0`, `area:plugin`.

### PL-02 — Measure Claude lifecycle edge cases

- **Goal:** Produce the ship-decision evidence for interrupts, multi-tool bursts, repeats, subagents, failures and refit interaction.
- **Owner:** `Lifecycle feasibility`
- **Why:** A plausible hook can still be too repetitive or slow to retain.
- **Scope:** Prototype matrix, p50/p95 latency, briefs/turn, repeat rate and recursion/fail-open behavior.
- **Files affected:** Dated evidence record and raw receipts only.
- **Out of scope:** Shipped code.
- **Dependencies:** PL-01. BM-09 remains a separate input to DR-07, not a prerequisite for measuring lifecycle behavior.
- **Acceptance criteria:** DR-07 has every required datapoint and known limitation.
- **Validation:** Repeat each scenario and independently inspect event counts.
- **Estimated complexity:** 60–90 minutes active.
- **Labels:** `needs-triage`, `type:research`, `priority:p0`, `area:plugin`.

### PL-03 — Implement Claude end-of-turn dedupe and invocation (**gated**)

- **Goal:** Add the approved full-check invocation once per eligible end-of-turn change.
- **Owner:** `Integration packaging`
- **Why:** This is the first bounded P0-1 retention implementation after evidence passes.
- **Scope:** Hook handler, change fingerprint/dedupe, CLI invocation and fail-open behavior.
- **Files affected:** `crates/argot-cli/src/` hook/lifecycle modules and tests.
- **Out of scope:** Manifest defaults, opt-out UX or other agents.
- **Dependencies:** DR-07 must say ship; HK-04, CLI-05/06.
- **Acceptance criteria:** Eligible changed turns invoke once; clean/no-change/repeat/error states remain quiet/non-blocking.
- **Validation:** Unit/integration fixtures from prototype scenarios.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:plugin`.

### PL-04 — Package lifecycle manifest and opt-out (**gated**)

- **Goal:** Wire the approved lifecycle into the Claude plugin with an explicit opt-out.
- **Owner:** `Integration packaging`
- **Why:** Product behavior must be inspectable and reversible by the user.
- **Scope:** `hooks/hooks.json`, plugin metadata/config and lifecycle-specific documentation pointer.
- **Files affected:** `hooks/hooks.json`, `.claude-plugin/*.json` and config test fixtures.
- **Out of scope:** Handler logic or broad claims.
- **Dependencies:** PL-03.
- **Acceptance criteria:** Fresh plugin install enables exactly the approved event; opt-out disables it; pre-write ask remains distinct.
- **Validation:** Manifest/schema and install fixture.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:plugin`.

### PL-05 — Add Claude lifecycle end-to-end matrix (**gated**)

- **Goal:** Prove the packaged plugin’s clean/noisy/unfitted/error/interrupt/repeat behavior.
- **Owner:** `Integration packaging`
- **Why:** Shipping a manifest without lifecycle E2E evidence would not close P0-1.
- **Scope:** Pinned-version manual/automated receipts and latency assertion.
- **Files affected:** Plugin test fixtures/workflow and evidence receipt.
- **Out of scope:** Other agents.
- **Dependencies:** PL-04.
- **Acceptance criteria:** All DR-07 behaviors pass and combined-noise canary uses the shipped package.
- **Validation:** End-to-end matrix receipt.
- **Estimated complexity:** 60–90 minutes active.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:plugin`.

### PL-06 — Add plugin package contract smoke

- **Goal:** Validate six skills, MCP startup, hook paths, versions and duplicate wiring.
- **Owner:** `Integration packaging`
- **Why:** The current bundle has five/six documentation drift and no package-level contract test.
- **Scope:** Plugin/marketplace manifests, skills paths/count, MCP command, pre-write and optional shipped lifecycle declaration.
- **Files affected:** Plugin test tooling/fixtures; manifests only for defects found within scope.
- **Out of scope:** Per-host ecosystem testing.
- **Dependencies:** HK-04; PL-05 only if lifecycle ships.
- **Acceptance criteria:** All declared assets exist/parse/start and version fields agree.
- **Validation:** Package smoke in CI or documented local runner.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p1`, `area:plugin`.

### MC-01 — Correct MCP tool capability descriptions

- **Goal:** State passive invocation and base-hunk coverage in tool/startup text.
- **Owner:** `Rust check contracts`
- **Why:** MCP check/explain are not the full production changeset composition.
- **Scope:** Five tool descriptions, startup instructions and full-CLI pointer; keep tool names.
- **Files affected:** `crates/argot-cli/src/mcp.rs`, `.mcp.json` if descriptive text exists.
- **Out of scope:** Expanding MCP detector coverage.
- **Dependencies:** CL-01.
- **Acceptance criteria:** No text implies guaranteed invocation/full detector coverage; CLI is named for complete changeset checking.
- **Validation:** Protocol snapshot and phrase audit.
- **Estimated complexity:** 30–45 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:plugin`.

### MC-02 — Add MCP protocol wording snapshots

- **Goal:** Lock tool names, capability boundaries and startup fit status.
- **Owner:** `Rust check contracts`
- **Why:** MCP copy is consumed by agents and easy to overstate silently.
- **Scope:** Initialize/list-tools/tool-description snapshots and fitted/unfitted startup text.
- **Files affected:** MCP tests/fixtures only.
- **Out of scope:** Tool implementation.
- **Dependencies:** MC-01.
- **Acceptance criteria:** Stable names remain and approved descriptions are asserted.
- **Validation:** MCP JSON-RPC fixture suite.
- **Estimated complexity:** 30–45 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p1`, `area:plugin`.

### PC-01 — Implement the approved pre-commit default

- **Goal:** Make hook behavior match DR-06 while preserving explicit setup errors and gate option.
- **Owner:** `Integration packaging`
- **Why:** Current docs and shipped exit behavior conflict.
- **Scope:** Hook entry/wrapper/args and manifest naming required by the decision.
- **Files affected:** `.pre-commit-hooks.yaml` and minimal CLI/wrapper file if approved.
- **Out of scope:** General check semantics or CI Action.
- **Dependencies:** DR-06, CLI-02, CLI-05/06.
- **Acceptance criteria:** Findings/setup errors/gating behave exactly as the decision table.
- **Validation:** Local pre-commit fixture run.
- **Estimated complexity:** 45–75 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:plugin`.

### PC-02 — Add pre-commit behavior matrix

- **Goal:** Test clean, error, warn, unfitted and command-failure commit attempts.
- **Owner:** `Integration packaging`
- **Why:** A default-behavior change requires regression and migration evidence.
- **Scope:** Fixture repository and hook invocation tests.
- **Files affected:** Integration test fixtures/scripts only.
- **Out of scope:** Docs or behavior changes.
- **Dependencies:** PC-01.
- **Acceptance criteria:** Default and explicit gate paths match DR-06 in all five cases.
- **Validation:** Automated fixture suite and one real `pre-commit` run.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:plugin`.

### IN-01 — Add the structured integration capability source

- **Goal:** Encode the approved EV-01 matrix once for docs, CLI next actions, README and landing summaries.
- **Owner:** `Integration packaging`
- **Why:** Repeating integration claims causes “70+” and automation drift.
- **Scope:** Small structured data file with type, event, coverage, prerequisite, blocking default, tested version/date and canonical guide.
- **Files affected:** New data source under a neutral docs/landing-readable path plus schema test.
- **Out of scope:** Editing consumers.
- **Dependencies:** EV-01, PC-02, CI-02–05; PL-05 status represented, never assumed.
- **Acceptance criteria:** Every current route has one row and unsupported/future routes cannot appear as tested.
- **Validation:** Schema and link-target validation.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:plugin`.

### SK-01 — Fix skill inventory and version metadata

- **Goal:** Make README/plugin metadata agree that six skills ship.
- **Owner:** `Integration packaging`
- **Why:** Current five/six drift undermines package trust.
- **Scope:** Count/list/version/path corrections only.
- **Files affected:** `skills/README.md`, `skills/VERSION`, `.claude-plugin/*.json` if inconsistent.
- **Out of scope:** Skill workflow rewrites.
- **Dependencies:** PL-06.
- **Acceptance criteria:** Six names/paths and all version fields agree.
- **Validation:** Plugin contract smoke.
- **Estimated complexity:** 30 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:skills`.

### SK-02 — Make `argot-setup` audit-first

- **Goal:** Reorder the skill to proof → interpret → init/exclusions/fit → smoke → integration.
- **Owner:** `Integration packaging`
- **Why:** D2/D5 make audit the acquisition front door; current skill performs setup first.
- **Scope:** `skills/argot-setup/SKILL.md` only.
- **Files affected:** `skills/argot-setup/SKILL.md`.
- **Out of scope:** Weakening repository inspection or wiring a lifecycle automatically.
- **Dependencies:** AU-03, IN-01.
- **Acceptance criteria:** Fresh and fitted repo branches are explicit and current automation limits are stated.
- **Validation:** Execute skill steps against ON-01 fixture.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:skills`.

### SK-03 — Reconcile check and review skill semantics

- **Goal:** Make confidence, exit, locality/network and fit-basis instructions match released CLI behavior.
- **Owner:** `Integration packaging`
- **Why:** Agent instructions must not preserve resolved CLI contradictions.
- **Scope:** `argot-check` and `argot-review-pr` skills only.
- **Files affected:** Their two `SKILL.md` files.
- **Out of scope:** Setup/CI/custom-rule skills.
- **Dependencies:** CLI-02, CLI-05/06, CL-01.
- **Acceptance criteria:** Display/severity semantics and review `gh`/auth/network/base-fit boundaries are exact.
- **Validation:** Run referenced commands/help and skill lint.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:skills`.

### SK-04 — Reconcile CI and rule-authoring skills

- **Goal:** Update setup-CI, write-rule and suggest-rules links/counts without expanding their product role.
- **Owner:** `Integration packaging`
- **Why:** Rule codification is P2 and must not displace audit/check activation; CI behavior changed.
- **Scope:** Three skill files and their canonical-doc links.
- **Files affected:** `skills/argot-setup-ci/SKILL.md`, `argot-write-rule/SKILL.md`, `argot-suggest-rules/SKILL.md`.
- **Out of scope:** New rule generation or CI behavior.
- **Dependencies:** CI-06/07, DOC-09.
- **Acceptance criteria:** Commands, rules, severities, advisory defaults and links match release.
- **Validation:** Skill lint and command smoke.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p2`, `area:skills`.

## Benchmark and claim-data issues

### BM-01 — Define the public claim-manifest schema

- **Goal:** Specify source, revision, date, numerator, denominator, scope, qualifier and supersession fields.
- **Owner:** `Benchmark claims`
- **Why:** Numeric claims cannot be safely generated without provenance and wording boundaries.
- **Scope:** Machine-readable schema and one foreign-detector example; no disputed lineage selection.
- **Files affected:** New manifest/schema under `landing/src/data/` or `benchmarks/`, plus validation test.
- **Out of scope:** Editing public consumers or running benchmarks.
- **Dependencies:** None.
- **Acceptance criteria:** Schema rejects missing source/revision/scope/qualifier and recomputes percentages.
- **Validation:** Valid/invalid fixture tests.
- **Estimated complexity:** 60 minutes.
- **Labels:** `ready-for-agent`, `type:implementation`, `priority:p0`, `area:benchmarks`.

### BM-02 — Inventory foreign, semantic and architecture claim artifacts

- **Goal:** Populate candidate manifest records without selecting disputed public winners.
- **Owner:** `Benchmark claims`
- **Why:** DR-09 needs a complete lineage rather than scattered JSON and prose.
- **Scope:** Existing `foreign.json`, `semantic.json`, `arch.json`, `latest.json`, generation dates/commits and known superseded values.
- **Files affected:** Candidate records/data notes beside the manifest.
- **Out of scope:** Integrity data or public copy.
- **Dependencies:** BM-01.
- **Acceptance criteria:** Every current/stale public value maps to a candidate artifact or is marked unsupported.
- **Validation:** Recompute all percentages and compare file revisions.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:research`, `priority:p0`, `area:benchmarks`.

### BM-03 — Produce machine-readable integrity claim data

- **Goal:** Convert current integrity evidence generations into candidate structured records.
- **Owner:** `Benchmark claims`
- **Why:** Integrity lacks a canonical machine source and has 144/153 vs 155/164 denominator drift.
- **Scope:** Recall, controls, accepted-history gating false fires, corpus/language counts and source revisions.
- **Files affected:** New candidate integrity JSON and validation fixture.
- **Out of scope:** Selecting the public generation or rerunning the full harness.
- **Dependencies:** BM-01.
- **Acceptance criteria:** Both old/current evidence generations are represented with exact denominators and provenance.
- **Validation:** Hand recomputation against evidence documents.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:research`, `priority:p0`, `area:benchmarks`.

### BM-04 — Finalize the canonical claim manifest

- **Goal:** Encode DR-09’s selected lineages and explicit unavailable claims.
- **Owner:** `Benchmark claims`
- **Why:** Consumers need one approved data source, not candidate evidence.
- **Scope:** Final foreign/semantic/architecture/integrity/language/performance keys and superseded values.
- **Files affected:** Canonical manifest and source links only.
- **Out of scope:** Consumer code or combined briefing results.
- **Dependencies:** DR-09, EV-04; EV-04 may remain a pending performance entry until complete.
- **Acceptance criteria:** Every public detector claim has one canonical key and allowed qualifier or is unavailable.
- **Validation:** Schema, percentage and source-link checks.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:benchmarks`.

### BM-05 — Add claim-consumer and drift helpers

- **Goal:** Expose typed/generated claim values and fail when known stale denominators reappear.
- **Owner:** `Benchmark claims`
- **Why:** Hand synchronization caused the current drift.
- **Scope:** Consumer helper/generator, seeded mutation test and stale-value scan.
- **Files affected:** Landing data helper/build test and CI test wiring.
- **Out of scope:** Rewriting consumer prose/pages.
- **Dependencies:** BM-04.
- **Acceptance criteria:** Consumers can render value+qualifier from keys; manifest mutation updates/fails all relevant tests.
- **Validation:** Production Astro build and seeded drift test.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:benchmarks`.

### BM-06 — Add production-composition benchmark adapter

- **Goal:** Run the same detector composition as the distributed binary from the benchmark harness.
- **Owner:** `Benchmark claims`
- **Why:** P0-2 cannot be measured by reconstructing a partial detector set.
- **Scope:** Adapter/API between `argot-bench` and `argot-core/src/compose.rs`; one deterministic fixture.
- **Files affected:** `crates/argot-bench/`, minimal public composition API if required, focused tests.
- **Out of scope:** Corpus replay or aggregate reporting.
- **Dependencies:** EV-02, CLI-02.
- **Acceptance criteria:** Fixture output matches release-feature `argot check` for identical config/diff.
- **Validation:** Parity test against release composition.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:benchmarks`.

### BM-07 — Implement accepted-change replay input

- **Goal:** Feed pinned accepted changes and adjudication metadata into BM-06.
- **Owner:** `Benchmark claims`
- **Why:** The relevant denominator is actual accepted changes, not planted break fixtures alone.
- **Scope:** Manifest loader, diff/range materialization and raw finding record emission.
- **Files affected:** `crates/argot-bench/`, pinned benchmark manifest/fixtures.
- **Out of scope:** Aggregate metrics or full run.
- **Dependencies:** BM-06.
- **Acceptance criteria:** A small protocol sample replays deterministically and preserves repo/SHA/rule/severity/latency fields.
- **Validation:** Three-case dry run from EV-02.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:benchmarks`.

### BM-08 — Add combined-brief aggregation

- **Goal:** Compute per-rule and union findings/change, briefs/change, latency and adjudication totals.
- **Owner:** `Benchmark claims`
- **Why:** Detector-specific recall/false-fire tables do not answer the retention question.
- **Scope:** Aggregator and machine-readable report with false/true/uncertain separated.
- **Files affected:** `crates/argot-bench/`, result schema and tests.
- **Out of scope:** Full evaluation run or threshold changes.
- **Dependencies:** BM-07.
- **Acceptance criteria:** Hand-worked fixture totals match output and changing one rule exposes its marginal union contribution.
- **Validation:** Unit tests with known counts.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:benchmarks`.

### BM-09 — Run and publish the combined briefing evaluation

- **Goal:** Execute the frozen protocol and publish raw records, aggregate result and gate verdict.
- **Owner:** `Benchmark claims`
- **Why:** This is the evidence gate for any automatic lifecycle launch.
- **Scope:** Full pinned run, adjudication record, dated evidence document and canonical-manifest combined key or explicit failed/unmeasured status.
- **Files affected:** Benchmark result artifacts, `docs/research/evidence/` and manifest entry.
- **Out of scope:** Tuning/re-running to reverse an unfavorable verdict.
- **Dependencies:** BM-08, DR-03.
- **Acceptance criteria:** Report includes all protocol denominators, uncertainty, latency and pass/fail against predeclared gates.
- **Validation:** Re-run deterministic subset; independent aggregate recomputation.
- **Estimated complexity:** 60–90 minutes active, plus unattended runtime/adjudication scheduling.
- **Labels:** `needs-triage`, `type:research`, `priority:p0`, `area:benchmarks`.

## Landing website issues

All landing issues run serially under one landing owner. Where `en.ts` is shared, each issue owns named content keys and must not edit another issue’s section.

### LD-01 — Repair locale routing and hreflang generation

- **Goal:** Stop generating nonexistent `/fr/docs/` links and alternates.
- **Owner:** `Landing product`
- **Why:** Broken activation links are an immediate factual defect.
- **Scope:** Locale path helper/use sites, existing-route-aware hreflang and route assertions.
- **Files affected:** `landing/src/lib/`, `landing/src/layouts/Base.astro`, French route link call sites and tests.
- **Out of scope:** Translating docs or rewriting homepage copy.
- **Dependencies:** None.
- **Acceptance criteria:** All generated internal links resolve; only existing localized routes receive alternates.
- **Validation:** Production build and locale route crawl.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `ready-for-agent`, `type:implementation`, `priority:p0`, `area:landing`, `bug`.

### LD-02 — Correct landing factual metadata defects

- **Goal:** Fix integrity severity, CI-default wording, duplicate heading ID and JSON-LD language metadata.
- **Owner:** `Landing product`
- **Why:** These are current factual errors independent of repositioning.
- **Scope:** Exact affected content keys/component, benchmark heading ID and structured data.
- **Files affected:** `landing/src/i18n/en.ts`, matching current French factual key, `landing/src/pages/benchmarks.astro`, `landing/src/layouts/Base.astro`.
- **Out of scope:** Hero/funnel/benchmark number rewrite.
- **Dependencies:** CL-01 for wording; route fix independent but owned by LD-01.
- **Acceptance criteria:** Only `test-weakened` is described default-warn; Action is non-blocking by default; IDs unique; JSON-LD describes the Rust product/multi-language analyzer.
- **Validation:** Astro build, DOM ID and metadata snapshot.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:landing`.

### LD-03 — Add landing claim-data consumers

- **Goal:** Replace hand-entered benchmark values with BM-05 keys.
- **Owner:** `Landing product`
- **Why:** Consumer copy must not drift from canonical evidence.
- **Scope:** Homepage proof/benchmark data plumbing only; retain existing layout until LD-08.
- **Files affected:** `landing/src/i18n/en.ts` numeric fields, benchmark components/page and data helper imports.
- **Out of scope:** New wording/layout or combined result presentation.
- **Dependencies:** BM-05.
- **Acceptance criteria:** No public detector number is entered outside the manifest and qualifiers travel with values.
- **Validation:** Drift test and production build.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:landing`.

### LD-04 — Replace English hero and primary CTA

- **Goal:** Lead with behavioral truth/product job and make audit the first action.
- **Owner:** `Landing product`
- **Why:** D1, D2, D10 and D11 require a behavior-led, layered explanation.
- **Scope:** Hero eyebrow/title/body/primary-secondary CTA/install note and homepage meta title/description.
- **Files affected:** Hero/meta keys in `landing/src/i18n/en.ts`, `Hero.astro` and CTA wiring.
- **Out of scope:** French copy, full page ordering or automatic-current claims.
- **Dependencies:** CL-01, EV-04.
- **Acceptance criteria:** One job and one first command are clear above fold; current lifecycle boundary is adjacent; formula is demoted from explanatory load.
- **Validation:** Claim review and desktop/mobile snapshot.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:landing`.

### LD-05 — Replace the behavioral-problem example

- **Goal:** Show one valid-looking change, repository evidence and human decision.
- **Owner:** `Landing product`
- **Why:** D11’s memorable-proof layer should demonstrate the behavioral problem without generic AI-review prose.
- **Scope:** Demo/Trust content keys and one fixture-backed component state; label authored vs wild.
- **Files affected:** Relevant landing component and its `en.ts` keys.
- **Out of scope:** Audit report proof or wild-case page.
- **Dependencies:** CL-01 and AS-01 or explicitly authored fixture receipt.
- **Acceptance criteria:** Example names rule/location/evidence and never calls a finding a proven bug.
- **Validation:** Fixture snapshot and a11y review of code/evidence.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:landing`.

### LD-06 — Reorder homepage around audit acquisition

- **Goal:** Produce the target problem → audit → evidence → habit → detail page order.
- **Owner:** `Landing product`
- **Why:** D2/D3/D5 make audit the front door; current film/feature tour delays it.
- **Scope:** `HomePage.astro` assembly/order and obsolete section removal/demotion only.
- **Files affected:** `landing/src/components/HomePage.astro`.
- **Out of scope:** Section copy/content or film implementation.
- **Dependencies:** LD-04, LD-05, DR-11 disposition known.
- **Acceptance criteria:** Hero/problem/audit are first; custom rules/engine/CI detail are secondary; no duplicate section anchors.
- **Validation:** Page outline snapshot and navigation link check.
- **Estimated complexity:** 30–45 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:landing`.

### LD-07 — Replace the homepage audit proof

- **Goal:** Render the reproducible audit artifact and bounded method note.
- **Owner:** `Landing product`
- **Why:** The current terminal is hand-authored and omits important boundaries.
- **Scope:** Audit component data/output, net-window/marker caveat and audit CTA.
- **Files affected:** `landing/src/components/Audit.astro`, audit keys in `en.ts`, generated asset/data import.
- **Out of scope:** Creating the artifact or integration section.
- **Dependencies:** AS-02, AU-04, EV-04.
- **Acceptance criteria:** Displayed output ties to pinned command/version/repo and links the canonical audit guide.
- **Validation:** Receipt-drift test and visual review.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:landing`.

### LD-08 — Add audit-to-habit and capability sections

- **Goal:** Show init plus real recurring choices with execution-class labels.
- **Owner:** `Landing product`
- **Why:** D2/D8 require the handoff while current support is easily overstated.
- **Scope:** Setup/integration section replacement using IN-01; automatic/passive/invoked/commit/CI labels.
- **Files affected:** Setup/CI/integration components and their `en.ts` keys.
- **Out of scope:** Full setup instructions or unsupported hosts.
- **Dependencies:** IN-01, AU-03; PL-05 status represented exactly.
- **Acceptance criteria:** User can choose a tested path and explain its trigger/coverage/default blocking behavior.
- **Validation:** Data-key/link tests and task-flow review.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:landing`.

### LD-09 — Rebuild benchmark and evidence page content

- **Goal:** Present detector-specific evidence and combined-result/unmeasured state with visible provenance.
- **Owner:** `Landing product`
- **Why:** D11 keeps proof distinct and D14 requires detector/noise claims to preserve their measured scope.
- **Scope:** Benchmark card text, integrity scorecard, method/source/date/denominator labels and combined section.
- **Files affected:** `landing/src/pages/benchmarks.astro`, benchmark/proof components and keys.
- **Out of scope:** Data generation or homepage layout.
- **Dependencies:** LD-03, BM-09.
- **Acceptance criteria:** Every metric shows scope/revision/source; blind spots and failed/unmeasured combined gate are visible.
- **Validation:** Manifest text assertions and build.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:landing`.

### LD-10 — Reframe install, privacy and open-source trust

- **Goal:** State free local core, no account/default telemetry, exact network paths and tested platforms.
- **Owner:** `Landing product`
- **Why:** D7, D12 and D13 require precise trust boundaries.
- **Scope:** Install/trust/CTA/footer/privacy summary keys and links.
- **Files affected:** Relevant homepage/footer components and `landing/src/i18n/en.ts`; the canonical privacy page remains DOC-11-owned.
- **Out of scope:** Full security/threat-model docs or pricing.
- **Dependencies:** CL-01, CI-02–05, DOC-11.
- **Acceptance criteria:** No absolute no-model/no-network/static claim remains; one-time model/version GET/offline boundaries are clear.
- **Validation:** Claim dictionary test and platform matrix link.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:landing`.

### LD-11 — Implement film disposition and social metadata

- **Goal:** Execute DR-11 and replace stale OG/social claims.
- **Owner:** `Landing product`
- **Why:** Film/OG currently carry voice/no-model/safety language and accessibility debt.
- **Scope:** Retain/update/demote/remove film as decided; metadata and language-neutral or EN/FR OG asset policy; sitemap indexing decision.
- **Files affected:** `Film.astro`, film assets/references, `Base.astro`, `landing/public/og.png`, sitemap config.
- **Out of scope:** Re-deciding the film or unrelated visual redesign.
- **Dependencies:** DR-11, LD-04, AS-03.
- **Acceptance criteria:** Film meets decided provenance/caption requirements or is absent; metadata matches current claims and proof routes have explicit indexing.
- **Validation:** Metadata snapshots, sitemap test and media keyboard review.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:landing`, `area:assets`.

### LD-12 — Translate stable landing content to French

- **Goal:** Bring the completed English funnel to claim and route parity.
- **Owner:** `Landing product`
- **Why:** Derived public surfaces cannot preserve stale positioning or broken links.
- **Scope:** `fr.ts` and French page-specific metadata/links only.
- **Files affected:** `landing/src/i18n/fr.ts`, `landing/src/pages/fr/*`.
- **Out of scope:** Translating English documentation.
- **Dependencies:** LD-04–11 stable.
- **Acceptance criteria:** French keys match capability/data sources, retain caveats and link only to existing routes.
- **Validation:** Locale crawl, structured key parity and native-language review.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:landing`.

### LD-13 — Add skip link and accessible mobile navigation

- **Goal:** Make global navigation usable on small screens and by keyboard/screen-reader users.
- **Owner:** `Landing product`
- **Why:** Current mobile navigation hides section links and no skip link exists.
- **Scope:** Skip target/link, mobile menu semantics, focus order, escape/close and visible focus.
- **Files affected:** `landing/src/components/Nav.astro`, base/page layout and global focus styles.
- **Out of scope:** Film modal or automated test infrastructure.
- **Dependencies:** LD-06.
- **Acceptance criteria:** All primary routes/actions are keyboard reachable at 320px; skip link bypasses navigation; focus is visible and restored.
- **Validation:** Keyboard-only and screen-reader spot check at desktop/mobile widths.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:landing`.

### LD-14 — Complete retained-film modal accessibility

- **Goal:** Add focus containment/restoration, background inert behavior and accessible media alternatives if the film remains.
- **Owner:** `Landing product`
- **Why:** Current modal has incomplete focus handling and no captions/transcript.
- **Scope:** Modal only, conditional on DR-11 retain/update.
- **Files affected:** `landing/src/components/Film.astro` and retained caption/transcript assets.
- **Out of scope:** Film claim/content editing or navigation.
- **Dependencies:** DR-11, LD-11.
- **Acceptance criteria:** Focus cannot escape, returns on close, background is inert, escape works and captions/transcript are reachable; issue closes as not-applicable if film is removed.
- **Validation:** Keyboard, screen-reader and reduced-motion test.
- **Estimated complexity:** 45–75 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:landing`, `area:assets`.

### LD-15 — Add production build, route and link gates

- **Goal:** Fail CI on broken internal routes, Markdown twins, locale links, hreflang and sitemap entries.
- **Owner:** `Landing product`
- **Why:** Current landing CI does not run the full production build or a route crawl.
- **Scope:** Build command, static output crawl and route/locale/sitemap assertions.
- **Files affected:** `landing/package.json`, test tooling and `.github/workflows/ci.yml` landing job.
- **Out of scope:** Accessibility or visual regression.
- **Dependencies:** LD-01–12.
- **Acceptance criteria:** CI runs production build and catches a seeded broken route/alternate/sitemap case.
- **Validation:** `just landing-check`, `just landing-build` and seeded negative test.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:landing`.

### LD-16 — Add axe/Lighthouse and responsive smoke

- **Goal:** Automate representative accessibility/performance checks and record the manual visual matrix.
- **Owner:** `Landing product`
- **Why:** Static inspection cannot validate rendered focus, overflow or modal behavior.
- **Scope:** Home, docs start, audit, integrations, benchmarks, proof and privacy at representative widths/reduced motion; 200% zoom manual record.
- **Files affected:** Landing test config/scripts and CI job only.
- **Out of scope:** Fixes discovered by the audit; file separate scoped issues if they exceed 30 minutes.
- **Dependencies:** LD-13–15, DOC-01–14.
- **Acceptance criteria:** No serious axe issues; budgets/checks are documented; 320/375/768/1440 and reduced-motion matrix is recorded.
- **Validation:** Automated report artifacts plus manual checklist.
- **Estimated complexity:** 60–90 minutes active.
- **Labels:** `needs-triage`, `type:qa`, `priority:p1`, `area:landing`.

## README issues

All README issues are serial work by one owner in one PR. Each issue owns named sections and must start from the previous issue’s commit.

### RD-01 — Replace README opening and audit-first quick start

- **Goal:** Make the first screen one product job, one current boundary and one executable audit path.
- **Owner:** `README`
- **Why:** D1/D2/D10/D11 reject the current voice/harness opening and feature-first flow.
- **Scope:** Logo/title/subtitle/badges, opening paragraphs, install and quick-start through audit next action.
- **Files affected:** `README.md` from top through quick start only.
- **Out of scope:** Integration, benchmark/privacy and proof/contribution sections.
- **Dependencies:** CL-01, AU-03, EV-04, IN-01.
- **Acceptance criteria:** Audit is above fold; no absolute local/no-model badge remains; cost/window/model boundaries are concise.
- **Validation:** Execute commands in ON-01 and render Markdown.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:readme`.

### RD-02 — Replace README integration section

- **Goal:** Summarize manual CLI, skills, MCP, Claude, pre-commit and Action from IN-01.
- **Owner:** `README`
- **Why:** “70+ agents” currently conflates installer reach with tested automatic support.
- **Scope:** Integration/setup sections and canonical guide links only.
- **Files affected:** `README.md` integration-related sections.
- **Out of scope:** Per-host setup instructions or automatic claims not marked shipped.
- **Dependencies:** RD-01, IN-01, PL-05 status.
- **Acceptance criteria:** Each route has execution class, prerequisite, coverage and tested status; no duplicated setup prose.
- **Validation:** Data/link check and claim audit.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:readme`.

### RD-03 — Reconcile README benchmarks, privacy, platforms and limitations

- **Goal:** Replace stale values and absolute technical claims with manifest-backed concise summaries.
- **Owner:** `README`
- **Why:** D7/D12/D13/D14 require precise proof/trust boundaries.
- **Scope:** Benchmark table, architecture/privacy/platform/limitations paragraphs and canonical links.
- **Files affected:** `README.md` named sections only.
- **Out of scope:** Full methodology or integration/proof sections.
- **Dependencies:** RD-02, BM-05, BM-09, EV-04, CI-02–05, DOC-10–13.
- **Acceptance criteria:** All numbers use manifest keys; 12 languages and five tested targets are accurate; local/model/network and blind spots are qualified.
- **Validation:** Claim/number drift and link tests.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:readme`.

### RD-04 — Refresh README proof and open-source links

- **Goal:** Link reproducible assets and make license/contribution/security/strategy routes obvious.
- **Owner:** `README`
- **Why:** D11 favors a separately evidenced proof layer over adjectives and Argot is explicitly an open-source product.
- **Scope:** Demo/proof/caught links, limitations pointer, MIT/contributing/security/canonical strategy links.
- **Files affected:** Remaining `README.md` proof/footer/acknowledgement sections.
- **Out of scope:** New assets or strategy duplication.
- **Dependencies:** RD-03, AS-02–05, DOC-15.
- **Acceptance criteria:** Every retained visual/case has provenance/regeneration link and authored/wild labels are explicit.
- **Validation:** Asset/link existence and Markdown render.
- **Estimated complexity:** 30–45 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:readme`.

## User and contributor documentation issues

### DOC-01 — Implement target docs navigation and route compatibility

- **Goal:** Create Start, Use, Configure, Understand and Help groups with stable routes.
- **Owner:** `Documentation journeys`
- **Why:** Current monoliths obscure the audit-to-habit journey.
- **Scope:** Frontmatter/order/sidebar, new route placeholders and redirects/compatibility pages for changed URLs.
- **Files affected:** `landing/src/layouts/DocsLayout.astro`, docs frontmatter and route files/placeholders.
- **Out of scope:** Page-body rewrites.
- **Dependencies:** None.
- **Acceptance criteria:** Target route map exists, old inbound links resolve and each topic has one canonical owner.
- **Validation:** Astro build and route crawl.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `ready-for-agent`, `type:docs`, `priority:p1`, `area:docs`.

### DOC-02 — Rewrite Getting Started

- **Goal:** Make docs entry install → audit → interpret → init → recurring choice.
- **Owner:** `Documentation journeys`
- **Why:** D2/D5 and P1-1 require proof before setup friction.
- **Scope:** `landing/src/content/docs/getting-started.md` only.
- **Files affected:** That page and its metadata.
- **Out of scope:** Detailed audit/init/integration procedures.
- **Dependencies:** DOC-01, RD-01, IN-01.
- **Acceptance criteria:** Fresh-clone commands pass ON-01 and every next step links one canonical guide.
- **Validation:** Execute command sequence and link check.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:docs`.

### DOC-03 — Create canonical Audit guide

- **Goal:** Explain audit purpose, formats, method, limits, timing and next actions once.
- **Owner:** `Documentation journeys`
- **Why:** D11’s proof layer and current facts are scattered across commands, README and research evidence.
- **Scope:** New Audit page and removal/link replacement of duplicated audit prose in command reference.
- **Files affected:** New audit content page; audit section in `the-commands.md` only.
- **Out of scope:** Research chronology or output implementation.
- **Dependencies:** DOC-01, AU-04, EV-04, AS-02.
- **Acceptance criteria:** Covers 50/cap1000, first-parent/base, net diff, marker floor, exit 0, formats, costs and habit CTA.
- **Validation:** Help/output parity and link check.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:docs`.

### DOC-04 — Create canonical Init and Fit guide

- **Goal:** Distinguish portable `init` setup from local-artifact-only `fit` refresh.
- **Owner:** `Documentation journeys`
- **Why:** Current prose can imply fit writes config and hides branch/dirty/health constraints.
- **Scope:** New guide; refactor setup and health/freshness links without duplicating bodies.
- **Files affected:** New init-fit page, `setup.md`, `health-and-freshness.md` relevant sections.
- **Out of scope:** Behavior changes or agent-specific integration setup.
- **Dependencies:** DOC-01, CLI-09, SK-02.
- **Acceptance criteria:** Mutations, default-branch/dirty/exclusion/inspect/refit/offline behavior are exact; no page says fit creates `argot.toml`.
- **Validation:** Source/help comparison and fixture walkthrough.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:docs`.

### DOC-05 — Create canonical Check and briefing guide

- **Goal:** Document full changeset scope, human brief, exit behavior, suppressions and machine formats.
- **Owner:** `Documentation journeys`
- **Why:** P1-2 and P2-2 require one current contract.
- **Scope:** New Check page; reduce `reading-the-output.md` and check section of command reference to links/reference detail.
- **Files affected:** New page, `reading-the-output.md`, `the-commands.md` check section.
- **Out of scope:** Agent lifecycle installation.
- **Dependencies:** DOC-01, CLI-02–06, CLI-10, CLI-04.
- **Acceptance criteria:** Worktree/staged/unstaged/commit/net-range, severity/confidence, clean boundary and schema links match release.
- **Validation:** Doctest-style snapshots and schema/link validation.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:docs`.

### DOC-06 — Consolidate Claude Code guide

- **Goal:** Give one tested path for binary, plugin, skills, MCP, pre-write and optional shipped end-of-turn behavior.
- **Owner:** `Documentation journeys`
- **Why:** Existing `plugin.md` and agent prose overlap and blur automatic coverage.
- **Scope:** `plugin.md` becomes canonical Claude guide; duplicate Claude sections elsewhere link to it.
- **Files affected:** `landing/src/content/docs/plugin.md`; Claude subsection links in `agents.md` only.
- **Out of scope:** Other agents or CI/pre-commit.
- **Dependencies:** DOC-01, PL-06, IN-01.
- **Acceptance criteria:** Capability table precedes setup; duplicate-hook warning, prerequisites, coverage, opt-out/update/uninstall are exact.
- **Validation:** Plugin journey and links.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:docs`.

### DOC-07 — Rewrite Other agents and MCP guide

- **Goal:** Explain generic skills/MCP compatibility without implying tested lifecycle automation.
- **Owner:** `Documentation journeys`
- **Why:** “70+” and passive MCP currently read too broadly.
- **Scope:** `agents.md` non-Claude content, MCP setup/coverage, named host status from IN-01.
- **Files affected:** `landing/src/content/docs/agents.md` only after DOC-06 link boundary.
- **Out of scope:** Claude details, hooks/pre-commit or new integrations.
- **Dependencies:** DOC-06, MC-02, IN-01, SK-01/03.
- **Acceptance criteria:** Every named host has tested/date/status or is generic; CLI is identified as full check.
- **Validation:** Config snippet and current-vendor-link checks.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:docs`.

### DOC-08 — Reconcile Action and pre-commit guide

- **Goal:** Document exact CI/base-fit/default-gate and commit-hook behavior.
- **Owner:** `Documentation journeys`
- **Why:** Current `ci.md` contradicts pre-commit behavior and overstates Action certainty.
- **Scope:** `ci.md` only: Action inputs/permissions/cache/SARIF/comment, other-CI recipe, pre-commit install/uninstall and explicit gate.
- **Files affected:** `landing/src/content/docs/ci.md`.
- **Out of scope:** Behavior/manifests or other agent docs.
- **Dependencies:** CI-07, PC-02, IN-01.
- **Acceptance criteria:** Every example is exercised; default/opt-in gate, network/offline and base-fit behavior match release.
- **Validation:** YAML/example smoke and link check.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:docs`.

### DOC-09 — Reconcile configuration and rules reference

- **Goal:** Make portable/local config, rule registry, migrations, exclusions and custom-rule behavior canonical.
- **Owner:** `Documentation reference`
- **Why:** Hook parity and live rule semantics must be reflected once without governance positioning.
- **Scope:** Configuration/rules/migrations/excludes/locks portions of `configure.md` and `custom-rules.md`.
- **Files affected:** Those named pages only; suppression sections reserved for DOC-10.
- **Out of scope:** New rule features, broad excludes or suppression workflow.
- **Dependencies:** HK-04, CLI-02, SK-04.
- **Acceptance criteria:** Tables match `argot rules`; hook-supported config subset is explicit; `rule-tampered` remains a mechanism, not positioning.
- **Validation:** Config/custom-rule fixtures and command snapshots.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p2`, `area:docs`.

### DOC-10 — Consolidate suppression workflow

- **Goal:** Give one inspect → act/mute with reason → commit → review/prune path.
- **Owner:** `Documentation reference`
- **Why:** Suppression power is spread across long pages and must retain human-last-word safeguards.
- **Scope:** Inline/path/mute/local config/locked behavior and `list-mutes`/`review-mutes` documentation.
- **Files affected:** Suppression sections of `configure.md`, `reading-the-output.md` links and command reference.
- **Out of scope:** New suppression behavior or autonomous muting.
- **Dependencies:** DOC-09, CLI-02.
- **Acceptance criteria:** Every example executes; locked rules cannot be softened; reasons and prune lifecycle are explicit.
- **Validation:** Command fixture walkthrough and link check.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p2`, `area:docs`.

### DOC-11 — Reconcile privacy and security network boundaries

- **Goal:** Publish the same exact local-analysis, egress and offline inventory on all trust surfaces.
- **Owner:** `Documentation reference`
- **Why:** D12/D13 contradict current “no network by default” and “nothing leaves” wording.
- **Scope:** `landing/src/pages/privacy.astro`, `SECURITY.md` and network/process trust-boundary sections of threat model.
- **Files affected:** Those three files only.
- **Out of scope:** Architecture/scoring explanations or behavior changes.
- **Dependencies:** CL-01, EV-04.
- **Acceptance criteria:** Model fetch, version GET, review/update/CI paths, no telemetry/code upload and `ARGOT_OFFLINE=1` agree everywhere.
- **Validation:** Claim search and code-path review.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:docs`.

### DOC-12 — Correct architecture and limitations pages

- **Goal:** Describe the current Rust composition/pipeline and publish explicit detection/setup limits.
- **Owner:** `Documentation reference`
- **Why:** Current how-it-works prose overstates all detectors learned from history and retains old extract/monolith details.
- **Scope:** `how-it-works.md`, `the-scoring-model.md` nonnumeric architecture sections and new Limitations page.
- **Files affected:** Those pages only.
- **Out of scope:** Benchmark values, privacy text or research logs.
- **Dependencies:** DOC-01, CL-01.
- **Acceptance criteria:** Architecture matches `compose.rs`; local encoder/non-generative boundary is exact; masked/in-vocabulary/fit suitability/net-range limits are listed.
- **Validation:** Architecture review against source and claim search.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:docs`.

### DOC-13 — Reconcile benchmarks and performance docs

- **Goal:** Make claim-bearing docs consume the canonical manifest and combined result state.
- **Owner:** `Documentation reference`
- **Why:** Current catches/scoring/performance pages mix metric generations.
- **Scope:** `what-it-catches.md`, numeric sections of `the-scoring-model.md`, `performance.md` and benchmark methodology links.
- **Files affected:** Named pages only.
- **Out of scope:** Historical research evidence or landing benchmark layout.
- **Dependencies:** BM-05, BM-09, EV-04.
- **Acceptance criteria:** No detector-specific result is generalized; all values/revisions/time ranges come from manifest evidence.
- **Validation:** Claim drift and percentage tests plus link check.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:docs`.

### DOC-14 — Add troubleshooting guide

- **Goal:** Resolve common cold-path failures from one page.
- **Owner:** `Documentation reference`
- **Why:** P1-1 activation currently ends in scattered error notes.
- **Scope:** Shallow/no history, unsupported files, Not recommended, model/offline, stale fit, Action/plugin/pre-commit, update/uninstall.
- **Files affected:** New troubleshooting page and link additions only.
- **Out of scope:** Fixing underlying defects.
- **Dependencies:** DOC-02–13.
- **Acceptance criteria:** Each scenario has symptom, cause, safe command and escalation link; CLI-08 points here where appropriate.
- **Validation:** Reproduce representative scenarios and link check.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p1`, `area:docs`.

### DOC-15 — Repair contributor architecture documentation

- **Goal:** Correct crate boundaries, adapter path, command inventory and verification workflow.
- **Owner:** `Documentation reference`
- **Why:** `crates/README.md` and `CONTRIBUTING.md` describe obsolete ownership and can misroute agents.
- **Scope:** Root contributor docs and `crates/README.md`; no user-facing product copy beyond links.
- **Files affected:** `CONTRIBUTING.md`, `crates/README.md`, `docs/agents/domain.md` if needed.
- **Out of scope:** `AGENTS.md`, llms exports or research history.
- **Dependencies:** None.
- **Acceptance criteria:** Workspace crates/paths/commands match current tree; 12-language adapter path is correct.
- **Validation:** Path/command existence and `just` help comparison.
- **Estimated complexity:** 45–60 minutes.
- **Labels:** `ready-for-agent`, `type:docs`, `priority:p2`, `area:docs`.

### DOC-16 — Reconcile AGENTS and llms exports

- **Goal:** Make agent-facing public summaries current without duplicating mutable facts by hand.
- **Owner:** `Documentation reference`
- **Why:** `llms.txt` says 11 languages and stale detector/voice claims; AGENTS is a major public integration surface.
- **Scope:** `AGENTS.md` public explanatory sections, `landing/src/pages/llms.txt.ts`, `llms-full.txt.ts` and generated-claim/integration consumption.
- **Files affected:** Named files only.
- **Out of scope:** Repository working instructions unrelated to product claims.
- **Dependencies:** BM-05, IN-01, SK-01–04, DOC-02–14.
- **Acceptance criteria:** 12 languages, audit-first flow, capability boundaries and current metrics are generated/linked from canonical sources.
- **Validation:** Generated output snapshots, claim audit and link check.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:docs`.

## Proof, onboarding and release issues

### AS-01 — Create the authored behavioral-problem fixture

- **Goal:** Produce one deterministic valid-looking change with real repository-grounded evidence for the homepage example.
- **Owner:** `Proof assets`
- **Why:** D11’s memorable-proof layer must be reproducible and labeled authored when it is not wild.
- **Scope:** Fixture repository/history, planted change, exact Argot command/output and regeneration note.
- **Files affected:** New fixture/receipt under `docs/demo/` or benchmark fixture conventions.
- **Out of scope:** Landing rendering or claiming a real-world catch.
- **Dependencies:** CLI-05/06.
- **Acceptance criteria:** One command reproduces the finding/hash/evidence and clearly labels the fixture authored.
- **Validation:** Deterministic rerun and snapshot.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:assets`.

### AS-02 — Generate a reproducible audit report bundle

- **Goal:** Commit pinned JSON, HTML/card and screenshot outputs with version/command provenance.
- **Owner:** `Proof assets`
- **Why:** Current landing audit simulation is hand-authored.
- **Scope:** Pinned redistributable repo/commit/window, generator, semantic on/off note and normalized dynamic fields.
- **Files affected:** `docs/demo/` generator/README and generated proof assets.
- **Out of scope:** Landing/README embedding or wild-catch claims.
- **Dependencies:** AU-04, EV-04.
- **Acceptance criteria:** One documented command regenerates every bundle artifact byte-for-byte or documents approved dynamic fields.
- **Validation:** CI receipt/snapshot check and visual inspection.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:assets`.

### AS-03 — Refresh the audit-first terminal recording and proof frames

- **Goal:** Replace check-first/stale visuals with the released audit-to-habit journey.
- **Owner:** `Proof assets`
- **Why:** Visual claims must match final CLI and public messaging.
- **Scope:** `docs/demo/demo.tape`, render script/GIF and reusable terminal/proof frames; recurring-lifecycle recording only if PL-05 shipped.
- **Files affected:** `docs/demo/*` and generated proof frames under the asset directory selected by LD-11; LD-11 alone owns final OG metadata/assets.
- **Out of scope:** CLI/landing copy or unverified wild stories.
- **Dependencies:** AS-02, CLI-09, AU-03, CL-01.
- **Acceptance criteria:** Assets name version/context, have regeneration steps and accessible alt/caption text; recurring recording names its exact event/agent.
- **Validation:** Render scripts, consumer search and visual review.
- **Estimated complexity:** 60–90 minutes active.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p1`, `area:assets`.

### AS-04 — Publish verified wild-case receipts

- **Goal:** Implement DR-10’s retained case/count decision with sourceable case data.
- **Owner:** `Proof assets`
- **Why:** Proof stories currently lack upstream links, real finding hashes and full corpus provenance.
- **Scope:** Case schema/data, receipts, upstream URLs, dates, reproduction/reconstruction labels and corrected hash display.
- **Files affected:** `landing/src/lib/caught-in-the-wild.ts`, proof pages and new receipt files.
- **Out of scope:** Maintaining unsupported counts or inventing private evidence.
- **Dependencies:** DR-10.
- **Acceptance criteria:** Every displayed fact is sourceable; unsupported total/case is absent; authored examples are labeled.
- **Validation:** Schema, links and reproduction checks.
- **Estimated complexity:** 60–90 minutes for the approved retained set; split again if DR-10 retains more than five verified cases.
- **Labels:** `needs-triage`, `type:implementation`, `priority:p0`, `area:assets`, `area:landing`.

### AS-05 — Remove orphaned assets and repair regeneration docs

- **Goal:** Ensure every committed public visual has a consumer and one accurate regeneration path.
- **Owner:** `Landing product`
- **Why:** `landing/public/demo.gif`, WebP references and demo-location prose currently drift.
- **Scope:** Asset consumer inventory, deletion of confirmed orphans and `docs/demo/README.md`/render instructions.
- **Files affected:** `landing/public/`, `docs/demo/README.md`, `docs/demo/render.sh` as needed.
- **Out of scope:** New proof content.
- **Dependencies:** AS-03/04, LD-11.
- **Acceptance criteria:** No orphan reference remains and no documented output format is missing.
- **Validation:** Repository asset-reference search and render smoke.
- **Estimated complexity:** 30–45 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p2`, `area:assets`.

### ON-01 — Add deterministic audit-to-habit journey fixture

- **Goal:** Exercise no-state install/binary → audit → init → check → chosen integration → finding → reasoned mute → rerun.
- **Owner:** `Release validation`
- **Why:** D5’s North Star exists only as disconnected commands today.
- **Scope:** Linux baseline fixture, normal/offline branches, mutation/network/exit-code receipt.
- **Files affected:** New integration/e2e fixture and script under existing test conventions.
- **Out of scope:** Five-platform matrix or every agent.
- **Dependencies:** AU-03, CLI-08/09, PC-02 or PL-05 selected integration, AS-01.
- **Acceptance criteria:** Flow starts without `.argot`, documents every mutation/network action and ends with expected suppressed/clean state.
- **Validation:** Automated Linux run with deterministic fixture.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p1`, `area:release`.

### EV-07 — Prototype one additional agent lifecycle (**later gated**)

- **Goal:** Test one non-Claude host only after the Claude path has retained acceptable signal.
- **Owner:** `Deferred plugin evidence`
- **Why:** P2-4 is allowed only after the shared foundation works; D6/D9 preserve optionality.
- **Scope:** Highest-confidence host from EV-01, pinned version, event/install/input/failure receipt and ship/reject recommendation.
- **Files affected:** Evidence/prototype files only.
- **Out of scope:** Shipping, multi-agent abstraction or broad support claim.
- **Dependencies:** PL-05 shipped, post-release canary passes, BM-09 remains within gate.
- **Acceptance criteria:** Exact lifecycle behavior and a named ship/reject outcome are documented.
- **Validation:** Same clean/noisy/unfitted/repeat/interrupt matrix as Claude.
- **Estimated complexity:** 60–90 minutes active.
- **Labels:** `needs-triage`, `type:research`, `priority:p3`, `area:plugin`.

### QA-01 — Run supported-platform clean-install journeys

- **Goal:** Extend ON-01’s product journey across the published installer target matrix.
- **Owner:** `Release validation`
- **Why:** Platform support claims require more than `argot --version` smoke.
- **Scope:** Available macOS/Linux/Windows runners; install, model fetch/offline, audit/init/check, update/uninstall ownership.
- **Files affected:** CI/manual receipts; workflow changes only if missing coverage is reusable.
- **Out of scope:** Other agents or load testing.
- **Dependencies:** ON-01, CI-02–05, PL-06.
- **Acceptance criteria:** Each claimed target has a passing journey or an explicit unverified limitation removed from claims.
- **Validation:** CI matrix plus manual receipt where interactive/plugin coverage is required.
- **Estimated complexity:** 60–90 minutes active plus CI runtime.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:release`.

### QA-02 — Run repository-wide public claim audit

- **Goal:** Classify every public occurrence as keep/rewrite/remove/qualify/internal/current/future against CL-01.
- **Owner:** `Release validation`
- **Why:** P0-3 spans Rust, Markdown, Astro, manifests, images and generated text.
- **Scope:** Text search, image/video review, numeric key use and explicit exception list.
- **Files affected:** Audit report under `docs/execution/`; small fixes must be returned to the owning issue/PR, not patched here.
- **Out of scope:** Strategy changes or new copy direction.
- **Dependencies:** All public area PRs merged; CL-01.
- **Acceptance criteria:** No forbidden current claim remains; every exception has path/reason/owner; unshipped lifecycle is future-tense.
- **Validation:** Repository grep, image OCR/manual inspection and generated-output samples.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:release`.

### REL-01 — Write compatibility and migration notes

- **Goal:** Document pre-commit, JSON, human-output, Action metric and plugin lifecycle changes for existing users.
- **Owner:** `Release validation`
- **Why:** These changes affect automation and need intentional upgrade guidance.
- **Scope:** Migration page/release-note source; unchanged guarantees and rollback/opt-out steps.
- **Files affected:** New release/migration document and GitHub release-note template/input.
- **Out of scope:** Hand-maintained per-version changelog duplication.
- **Dependencies:** DR-13 and final behavior PRs.
- **Acceptance criteria:** Every compatibility change has before/after, action required and canonical docs link.
- **Validation:** Upgrade previous release on a fixture and follow instructions.
- **Estimated complexity:** 60–90 minutes.
- **Labels:** `needs-triage`, `type:docs`, `priority:p0`, `area:release`.

### REL-02 — Add release version-consistency check

- **Goal:** Verify Cargo, plugin, skills, MCP registry, npm/site version and Action tag agree.
- **Owner:** `Release validation`
- **Why:** Multiple distribution surfaces can publish inconsistent releases.
- **Scope:** Script/test in release workflow before publication.
- **Files affected:** `.github/workflows/release.yml`/`auto-release.yml` and a small version-check script/test.
- **Out of scope:** Changing release cadence or version policy.
- **Dependencies:** PL-06, CI-02–05.
- **Acceptance criteria:** A seeded mismatch fails before publish and a consistent tree passes.
- **Validation:** Release workflow dry run/fixture.
- **Estimated complexity:** 60 minutes.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:release`.

### REL-03 — Execute lifecycle canary and record verdict

- **Goal:** Validate the packaged recurring lifecycle on a bounded cohort before public current-tense claims.
- **Owner:** `Release validation`
- **Why:** D14 requires real canary evidence after benchmark/prototype gates.
- **Scope:** Canary procedure, opt-out/failure monitoring without default telemetry, qualitative/explicit opt-in evidence and stop criteria.
- **Files affected:** Canary/release record only; no product fixes in this issue.
- **Out of scope:** Broad rollout or hidden instrumentation.
- **Dependencies:** DR-07 ship, PL-05, BM-09, DR-13.
- **Acceptance criteria:** Pass/fail/defer verdict is recorded against predeclared criteria; failure preserves manual/user-wired messaging.
- **Validation:** Receipt review and explicit participant consent/data boundary.
- **Estimated complexity:** 60–90 minutes active; elapsed canary time separate.
- **Labels:** `needs-triage`, `type:research`, `priority:p0`, `area:release`.

### REL-04 — Perform post-release distribution and claim smoke

- **Goal:** Verify released binaries, npm, plugin/skills/MCP, Action, website and docs against the exact tag.
- **Owner:** `Release validation`
- **Why:** Source-tree success does not prove the public release is coherent.
- **Scope:** Version/URL/install/action/plugin/site/claim smoke and final receipt.
- **Files affected:** Post-release evidence record; fixes go to owning area follow-ups.
- **Out of scope:** Feature work or strategy changes.
- **Dependencies:** QA-01/02, REL-01/02; REL-03 if lifecycle ships.
- **Acceptance criteria:** All surfaces report the same version; public commands/links work; only shipped claims appear.
- **Validation:** Tagged clean-install, Action/plugin smoke and website crawl.
- **Estimated complexity:** 60–90 minutes active.
- **Labels:** `needs-triage`, `type:qa`, `priority:p0`, `area:release`.

## Backlog priority summary

### P0 — Start or resolve first

- Decisions: DR-01–10, DR-13; DR-02/03/07 are the automatic-lifecycle gate chain.
- Immediate reliability/truth: CI-01–05, CLI-01/02/07/10, AU-01/02/04, MC-01, PC-01/02, BM-01–05, EV-01/05, CL-01, LD-01–04/06–10/15, RD-01–03, DOC-03/08/11–13/16.
- Retention evidence: EV-02/03, BM-06–09, PL-01/02.

### P1 — Activation and retained trust

- CLI-05/06/08/09, AU-03, HK-01–04, PL-06, IN-01, SK-01–03, LD-05/11–14/16, RD-04, DOC-01/02/04–07/14, AS-01/03, ON-01.

### P2 — Foundation/quality after launch blockers

- DR-04/05/11/14 where not already required, CLI-03/04, SK-04, DOC-09/10/15, AS-05.

### P3 — Do not schedule until gate crosses

- DR-12 and EV-07. Durable-history implementation issues do not exist until DR-12 approves a concrete local-only specification.
