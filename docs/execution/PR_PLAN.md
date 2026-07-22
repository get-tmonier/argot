# Argot final pull-request architecture

This document is the immutable integration design for
`GITHUB_BACKLOG.md`. Issues are acceptance units; the 15 PRs below are the only
implementation units. Every PR has one owner, one review topic and an exclusive
file lease.

All agents must follow `AGENT_EXECUTION_RULES.md`.

## Optimization audit

The previous 20-PR design was correct but not autonomous-agent-safe:

| Finding | Execution risk | Resolution |
| --- | --- | --- |
| Issue ownership was implicit | Agents could claim the same files from different labels | Added one explicit owner to every issue |
| Three serial Rust PRs shared help/snapshots | Rebase and CI churn | Combined check, machine-output and MCP contracts; kept audit activation separate |
| Evidence was split across two documentation PRs | A timing dependency forced a small extra PR | Moved EV-04 to the benchmark-data owner and consolidated all other evidence |
| Hook feasibility and Claude feasibility were split | Same lifecycle context and package fixtures were reloaded twice | Combined into one non-shipping lifecycle-feasibility PR |
| Plugin release and skills/pre-commit packaging were split | Shared manifests/version data created a likely conflict | Combined after DR-07 into one integration-packaging PR |
| Landing had three serial product PRs before README | Excessive site builds and rebases | Combined routing/data/content/localization; retained a separate Very High QA PR |
| EV-03 waited for DR-01 and PL-02 waited for BM-09 | Independent evidence collection was unnecessarily serialized across PRs | Removed those two dependencies; DR-14 and DR-07 still own the downstream decisions |
| Cross-surface issues had only area labels | CI-06/07, MC-01/02 and AS-05 could be claimed by competing areas | Assigned them to the single PR that owns their actual files |
| All 120 issues were rechecked for size and scope | A hidden umbrella issue would make the PR cap meaningless | Retained 30–90 minute units; AS-04 keeps its explicit split-at-five-cases stop rule |
| PR prerequisites were not uniformly “merged” gates | Agents could create stacked PRs | Every start gate below requires merged main; open PRs never satisfy it |
| No immutable-backlog or worktree contract | Autonomous agents could rewrite scope or collide | Added the execution lock and agent constitution |

No issue was merged, redefined or removed. The reorganization changes only PR
ownership and scheduling. All 120 issue bodies remain independent 30–90 minute
acceptance units.

## Global PR constraints

- Create a new, never-before-used worktree and `codex/` branch for exactly one
  PR. Delete the worktree after merge; never recycle it.
- Fetch the latest merged `main` before creating the worktree. An open PR never
  satisfies a prerequisite.
- Maximum approximately 60 files and 2,000 reviewed lines per PR, excluding
  generated assets. Stop and split the PR at an issue boundary before exceeding
  either cap.
- Only files in the declared lease may change. A required out-of-lease edit is a
  blocker and new issue, not an opportunistic fix.
- One commit per completed issue is preferred. All issue acceptance checks run
  locally before the single consolidated CI request.
- CI classes: **Low**, **Medium**, **High**, **Very High**. Only one Very High PR
  may execute CI at a time.
- PRs can merge together only when the merge groups below say so. The merge
  queue rechecks each PR against the resulting latest `main`.

## PR-01 — Distribution baseline

- **Owner:** Distribution.
- **Issues:** CI-01, CI-02, CI-03, CI-04, CI-05.
- **Review topic:** Released Action/installer artifact resolution and platform smoke.
- **Exclusive lease:** `action.yml`, distribution/install scripts and their
  dedicated workflow jobs/fixtures.
- **Forbidden overlap:** Rust renderers, landing, docs, README and release-note
  content.
- **Start gate:** None; branch from latest merged `main`.
- **Size budget:** ≤20 files; approximately 300–800 reviewed lines.
- **CI cost:** **High** — platform and negative-path jobs.
- **Local validation:** Action fixture, checksum/missing-asset cases and workflow
  syntax before requesting the platform matrix.
- **Merge:** Merge Group A; may merge concurrently with PR-02.

## PR-02 — Evidence and public-claim contract

- **Owner:** Evidence and claims.
- **Issues:** EV-01, EV-02, EV-03, EV-05, EV-06, CL-01.
- **Review topic:** Verified capability/proof evidence and one allowed/forbidden
  public-claim dictionary.
- **Exclusive lease:** Named new files under `docs/research/evidence/` and
  `docs/execution/PUBLIC_CLAIMS.md`.
- **Forbidden overlap:** Product code, strategy, execution-plan/backlog bodies,
  landing, README and benchmark implementation.
- **Start gate:** None. EV-01 completes before DR-02; DR-02 then unlocks EV-02/03
  inside this one owner lane.
- **Size budget:** ≤18 files; approximately 700–1,400 documentation lines plus receipts.
- **CI cost:** **Low** — links, fixtures and evidence checks.
- **Local validation:** Reproduction/link checks and strategy D-register audit.
- **Merge:** Merge Group A; may merge concurrently with PR-01.

## PR-03 — Rust check and machine contracts

- **Owner:** Rust check contracts.
- **Issues:** CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06, CLI-10, CLI-11,
  CI-06, CI-07, MC-01, MC-02.
- **Review topic:** Stable check semantics and honest human/machine descriptions.
- **Exclusive lease:** `crates/argot-engine/src/check/`, check JSON/output/schema
  code and fixtures, `crates/argot-cli/src/mcp.rs`, GitHub/SARIF summary renderer
  and related snapshots. `crates/argot-cli/src/main.rs` is leased only for check
  help text.
- **Forbidden overlap:** Audit modules, hook implementation, plugin packaging,
  Action install mechanics and public docs.
- **Start gate:** PR-02 merged; DR-02 and DR-14 resolved. CLI-01 may then inform
  DR-01; CLI-03/04 may inform DR-04/05; no dependent behavior is implemented
  until each decision closes.
- **Size budget:** ≤35 files; approximately 900–1,700 reviewed lines.
- **CI cost:** **Medium** — focused crates, schema fixtures and snapshots.
- **Local validation:** Focused Rust tests for every issue followed by one
  workspace contract run.
- **Merge:** Merge Group B; may merge concurrently with PR-04.

## PR-04 — Benchmark claim data

- **Owner:** Benchmark claims.
- **Issues:** EV-04, BM-01, BM-02, BM-03, BM-04, BM-05.
- **Review topic:** Canonical, provenance-bearing public metric data.
- **Exclusive lease:** Benchmark claim schema/candidate/canonical data, integrity
  claim data, timing evidence, claim-consumer helper and drift tests.
- **Forbidden overlap:** `crates/argot-bench/` combined harness, public page copy,
  strategy and detector tuning.
- **Start gate:** PR-01 merged. BM-01–03 complete before DR-09; BM-04/05 wait for
  DR-09 and EV-04 inside the same lane.
- **Size budget:** ≤25 files; approximately 600–1,200 reviewed lines/data records.
- **CI cost:** **Medium** — schema/drift tests and one production data-consumer build.
- **Local validation:** Recompute every percentage, validate source links and run
  seeded stale-value failures.
- **Merge:** Merge Group B; may merge concurrently with PR-03.

## PR-05 — Rust audit activation

- **Owner:** Rust audit activation.
- **Issues:** CLI-07, CLI-08, CLI-09, AU-01, AU-02, AU-03, AU-04.
- **Review topic:** Audit-first discovery and the audit → fit → recurring-check handoff.
- **Exclusive lease:** `crates/argot-cli/src/audit/`, root/audit/init/unfitted help
  and diagnostics, audit fixtures. `crates/argot-cli/src/main.rs` is exclusively
  leased by this PR after PR-03 merges.
- **Forbidden overlap:** Check render/schema internals, hook, MCP, public docs and assets.
- **Start gate:** PR-03 merged.
- **Size budget:** ≤25 files; approximately 700–1,400 reviewed lines.
- **CI cost:** **Medium** — audit and CLI snapshot/integration tests.
- **Local validation:** Audit contract fixtures, exit-0 verification and complete
  unfitted/init command flow.
- **Merge:** Merge Group C; may merge concurrently with PR-06 and PR-07.

## PR-06 — Hook and Claude lifecycle feasibility

- **Owner:** Lifecycle feasibility.
- **Issues:** HK-01, HK-02, HK-03, HK-04, PL-01, PL-02.
- **Review topic:** Accurate fail-open pre-write behavior and a non-shipping
  Claude end-of-turn feasibility verdict.
- **Exclusive lease:** `crates/argot-cli/src/hook.rs`, hook-only config adapters
  and tests, non-released Claude prototype harness and lifecycle evidence.
- **Forbidden overlap:** `main.rs`, check/audit renderers, released plugin
  manifest, skills, pre-commit and public current-tense claims.
- **Start gate:** PR-02 and PR-03 merged; DR-02 resolved.
- **Size budget:** ≤25 files; approximately 650–1,300 reviewed lines plus receipts.
- **CI cost:** **High** — hook matrix and pinned lifecycle smoke.
- **Local validation:** Clean/noisy/unfitted/repeated/interrupt/failure matrix;
  released plugin behavior must remain byte-for-byte unchanged.
- **Merge:** Merge Group C; may merge concurrently with PR-05 and PR-07.

## PR-07 — Combined accept-brief benchmark

- **Owner:** Benchmark claims.
- **Issues:** BM-06, BM-07, BM-08, BM-09.
- **Review topic:** Production-composition accepted-change replay and frozen gate verdict.
- **Exclusive lease:** `crates/argot-bench/`, the minimal composition seam approved
  by PR-03, pinned benchmark corpus/results and dated combined evidence.
- **Forbidden overlap:** Public copy, threshold tuning, unrelated detector code
  and benchmark claim-lineage data owned by PR-04.
- **Start gate:** PR-03 and PR-04 merged; DR-03 resolved.
- **Size budget:** ≤30 files; approximately 800–1,600 reviewed lines plus results.
- **CI cost:** **Very High** — deterministic subset in normal CI; full corpus via
  one manually dispatched artifact-producing run.
- **Local validation:** Release-composition parity, three-case replay,
  hand-computed aggregation and deterministic subset rerun.
- **Merge:** Merge Group C. It may be code-reviewed concurrently, but no other
  Very High CI run may overlap its full evaluation.

## PR-08 — Recurring integration packaging

- **Owner:** Integration packaging.
- **Issues:** PL-03, PL-04, PL-05, PL-06, PC-01, PC-02, IN-01, SK-01, SK-02,
  SK-03, SK-04.
- **Review topic:** One truthful package of tested recurring integration choices.
- **Exclusive lease:** `.claude-plugin/`, released plugin hook scripts/fixtures,
  `.pre-commit-hooks.yaml`, structured integration capability data and `skills/`.
- **Forbidden overlap:** Rust hook/check/audit code, docs consumers, landing,
  README and release workflow.
- **Start gate:** PR-05, PR-06 and PR-07 merged; DR-06 and DR-07 resolved. If
  DR-07 is defer/reject, PL-03/04/05 remain blocked and are not silently rewritten;
  the PR implements the remaining issues and records current limits in IN-01.
- **Size budget:** ≤45 files; approximately 900–1,700 reviewed lines.
- **CI cost:** **High** — plugin package, pre-commit and skill matrices.
- **Local validation:** Package smoke, exact lifecycle matrix when shipped,
  pre-commit cases and skill lint/command smoke.
- **Merge:** Merge Group D; may merge concurrently with PR-09.

## PR-09 — Reproducible proof assets

- **Owner:** Proof assets.
- **Issues:** AS-01, AS-02, AS-03, AS-04.
- **Review topic:** Reproducible authored, audit and verified-wild proof inputs.
- **Exclusive lease:** `docs/demo/`, new proof receipts/fixtures/media and
  `landing/src/lib/caught-in-the-wild.ts` plus its dedicated proof routes.
- **Forbidden overlap:** Homepage components/i18n, final OG metadata/assets,
  README and unsupported case totals.
- **Start gate:** PR-05 and PR-07 merged; DR-10 resolved.
- **Size budget:** ≤30 files; approximately 500–1,000 reviewed lines plus generated media.
- **CI cost:** **Medium** — deterministic regeneration, snapshots and links.
- **Local validation:** Rebuild assets from pinned commands, compare normalized
  outputs/checksums and verify every retained source URL.
- **Merge:** Merge Group D; may merge concurrently with PR-08.

## PR-10 — User journey documentation

- **Owner:** Documentation journeys.
- **Issues:** DOC-01, DOC-02, DOC-03, DOC-04, DOC-05, DOC-06, DOC-07, DOC-08.
- **Review topic:** One canonical audit-to-habit documentation journey.
- **Exclusive lease:** Docs navigation/route compatibility and the named getting
  started, audit, init/fit, check, Claude, other-agent/MCP and CI/pre-commit pages.
- **Forbidden overlap:** Reference/trust pages owned by PR-11, landing homepage,
  README and product code.
- **Start gate:** PR-08 merged; PR-05/07 already merged.
- **Size budget:** ≤40 files; approximately 1,100–1,900 reviewed lines.
- **CI cost:** **Medium** — docs production build, route aliases and link crawl.
- **Local validation:** Execute every documented command on fixtures, then build
  and crawl canonical/legacy routes.
- **Merge:** Merge Group E; exclusive docs ownership, so PR-11 starts only after merge.

## PR-11 — Trust and reference documentation

- **Owner:** Documentation reference.
- **Issues:** DOC-09, DOC-10, DOC-11, DOC-12, DOC-13, DOC-14, DOC-15, DOC-16.
- **Review topic:** Accurate configuration, trust, architecture, evidence and
  contributor reference.
- **Exclusive lease:** Named configure/rules/suppression/privacy/security/
  architecture/limitations/benchmark/performance/troubleshooting pages,
  `CONTRIBUTING.md`, `crates/README.md`, `AGENTS.md` and llms export generators.
- **Forbidden overlap:** Journey page bodies, landing homepage, README and strategy.
- **Start gate:** PR-04, PR-07, PR-08 and PR-10 merged.
- **Size budget:** ≤50 files; approximately 1,200–1,950 reviewed lines.
- **CI cost:** **Medium** — docs build, generated exports, links and claim drift.
- **Local validation:** Source-path/command checks, generated-export snapshots,
  network-boundary audit and benchmark-manifest assertions.
- **Merge:** Merge Group F; no concurrent docs owner.

## PR-12 — Landing repositioning

- **Owner:** Landing product.
- **Issues:** LD-01, LD-02, LD-03, LD-04, LD-05, LD-06, LD-07, LD-08, LD-09,
  LD-10, LD-11, LD-12.
- **Review topic:** Complete behavior-led, audit-first, evidence-backed public site.
- **Exclusive lease:** Landing routing/base metadata, homepage/product/evidence
  components, benchmark page consumers, English/French i18n and final film/OG policy.
- **Forbidden overlap:** Docs content/reference pages, proof source generation,
  README and landing CI/a11y tooling owned by PR-13.
- **Start gate:** PR-04, PR-08, PR-09, PR-10 and PR-11 merged; DR-11 and DR-13
  resolved. If automatic lifecycle shipped, REL-03 must pass before current-tense copy.
- **Size budget:** ≤55 files; approximately 1,100–1,950 reviewed lines excluding generated media.
- **CI cost:** **High** — production build, unit/snapshot and targeted visual checks.
- **Local validation:** Claim-data assertions, route/locale crawl, desktop/mobile
  snapshots and explicit current-versus-future claim scan.
- **Merge:** Merge Group G; exclusive landing ownership.

## PR-13 — Landing accessibility and release gates

- **Owner:** Landing product.
- **Issues:** LD-13, LD-14, LD-15, LD-16, AS-05.
- **Review topic:** Accessible, route-safe, reproducible landing release gates.
- **Exclusive lease:** Navigation/modal accessibility, landing test/build tooling,
  landing-only CI job and orphan landing/demo asset cleanup.
- **Forbidden overlap:** Product copy, docs, README and general distribution/release workflows.
- **Start gate:** PR-12 merged.
- **Size budget:** ≤30 files; approximately 500–1,000 reviewed lines plus deletions.
- **CI cost:** **Very High** — full production crawl, axe/Lighthouse and responsive matrix.
- **Local validation:** Keyboard/screen-reader/reduced-motion checks, seeded broken
  route and 320/375/768/1440 plus 200%-zoom matrix before CI.
- **Merge:** Merge Group H; may merge concurrently with PR-14, but it has the sole
  Very High CI lease.

## PR-14 — README public entry point

- **Owner:** README.
- **Issues:** RD-01, RD-02, RD-03, RD-04.
- **Review topic:** Concise audit-first repository entry point using shipped facts.
- **Exclusive lease:** Root `README.md` only; it may reference already-merged assets.
- **Forbidden overlap:** Every other file.
- **Start gate:** PR-12 merged; DR-13 resolved. PR-13 need not be open or merged.
- **Size budget:** 1 file; approximately 250–500 reviewed lines.
- **CI cost:** **Low** — Markdown, commands, links and claim checks.
- **Local validation:** Execute quick start, render Markdown and resolve every link/asset.
- **Merge:** Merge Group H; may merge concurrently with PR-13.

## PR-15 — Release-readiness automation and evidence

- **Owner:** Release validation.
- **Issues:** ON-01, QA-01, QA-02, REL-01, REL-02.
- **Review topic:** Exact clean-install journey, compatibility and claim release gate.
- **Exclusive lease:** End-to-end/release fixtures, release/version workflow and
  checker, migration/release-note source and final claim-audit report.
- **Forbidden overlap:** Opportunistic fixes in product, docs, landing, README,
  strategy or backlog. Failures create new area-owned issues.
- **Start gate:** PR-13 and PR-14 merged; every earlier applicable PR merged;
  DR-13 resolved and REL-03 passed when the automatic lifecycle ships.
- **Size budget:** ≤35 files; approximately 650–1,300 reviewed lines plus CI receipts.
- **CI cost:** **Very High** — supported-platform journey, release dry run and full crawl.
- **Local validation:** Linux deterministic journey, version mismatch fixture,
  claim scan and workflow dry-run before requesting the full matrix.
- **Merge:** Merge Group I; last pre-release source PR and sole Very High CI owner.

## No-PR issues

| Issues | Disposition |
| --- | --- |
| DR-01 through DR-14 | Human decisions recorded in GitHub; no placeholder source PR |
| REL-03 | Operational lifecycle canary after PR-08; required before an automatic current-tense claim |
| REL-04 | Tagged post-release distribution/claim smoke |
| EV-07 | Deferred evidence-only PR created only after its explicit gate crosses |

## Merge train

| Group | PRs | Can merge simultaneously? | Gate for next group |
| --- | --- | --- | --- |
| A | PR-01, PR-02 | Yes; disjoint distribution and evidence leases | Both merged |
| B | PR-03, PR-04 | Yes; Rust contracts and benchmark data are disjoint | Both merged |
| C | PR-05, PR-06, PR-07 | Yes; file leases are disjoint. PR-07 exclusively holds Very High CI | All merged; DR-07 resolved |
| D | PR-08, PR-09 | Yes; integration package and proof assets are disjoint | Both merged; REL-03 if applicable |
| E | PR-10 | No peer; exclusive journey-docs owner | Merged |
| F | PR-11 | No peer; follows PR-10 to avoid docs conflicts | Merged |
| G | PR-12 | No peer; follows canonical docs/proof/integration facts | Merged |
| H | PR-13, PR-14 | Yes; landing QA and README leases are disjoint. PR-13 holds Very High CI | Both merged |
| I | PR-15 | No; final pre-release gate and sole Very High CI owner | Release candidate approved |

Within a group, merge order is irrelevant. Between groups, later worktrees are
not created until all named prerequisites are merged. A PR is never based on an
open PR, never rebased onto an agent branch and never kept as a stack.

## Size and CI stop rules

- At 50 files or 1,700 estimated reviewed lines, the owner forecasts remaining
  scope. If the cap is likely to be exceeded, split at the next untouched issue
  boundary before editing more files.
- Generated binary/media outputs do not count toward reviewed lines, but their
  generator, provenance and checksum do.
- A PR moving from its assigned CI class to Very High must wait for the program
  manager to grant the single Very High CI lease.
- Failed consolidated CI is repaired only within the PR lease. An out-of-lease
  failure blocks the PR and creates a new issue for the owning area.
