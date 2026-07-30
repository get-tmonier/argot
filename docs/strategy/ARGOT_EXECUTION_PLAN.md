# Argot execution plan

**Status:** execution-ready repository plan

**Canonical source:** [`ARGOT_STRATEGY.md`](ARGOT_STRATEGY.md)

**Current-fact source:** [`ARGOT_CURRENT_REALITY.md`](ARGOT_CURRENT_REALITY.md)

**Known-gap source:** [`ARGOT_PRODUCT_GAPS.md`](ARGOT_PRODUCT_GAPS.md)

**Prepared from repository inspection:** 2026-07-22

This document turns the accepted strategy into ordered, independently executable work. It does not reopen positioning, make future product bets, or treat a planned capability as shipped. Paths are repository-relative unless stated otherwise.

## 1. Executive summary

Argot must move from explaining a statistical “voice linter” to delivering one clear job: make repository-grounded divergence visible when generated code is about to be accepted. The real zero-prior-fit `argot audit` is the acquisition front door; a quiet, concise, automatic check at the nearest reliable accept-time lifecycle is the retention target.

Four changes are required. First, measure the combined default briefing and prove that its noise is low enough. Second, ship and test one honest automatic lifecycle, beginning with the already supported Claude Code plugin only if its stop/end-of-turn prototype passes the product and noise gates. Third, make audit, setup, CLI output, and integrations form one continuous audit-to-habit journey. Fourth, replace conflicting claims across the landing site, README, docs, reports, metadata, skills, Action, and security/privacy surfaces with a single claim ledger backed by executable evidence.

The local-first analytical core, user-owned configuration, open-source identity, free individual local check, no-default-telemetry policy, and non-generative authoritative path must not change. “Voice” can remain a brand motif and in compatibility-sensitive names, but it cannot explain the product. Future dashboards, accounts, cloud, governance, or monetization are excluded until their strategy gates are crossed.

The intended end state is an honest funnel: discover the behavioral problem, run a credible audit, understand the evidence and limits, fit the repository, enable the best tested recurring workflow, receive a brief with evidence and an explicit human decision, and keep Argot enabled because it stays quiet when nothing important changed.

## 2. Strategy-to-execution translation

| Strategy decision | Execution principle | Observable end state |
| --- | --- | --- |
| D1 — awareness at acceptance | Design the product around the last reliable lifecycle before a user treats generated code as accepted; call a proxy a proxy. | At least one tested integration runs automatically at a documented lifecycle and presents an actionable, non-blocking brief. |
| D2 — acquisition and retention are separate | Do not overload audit with recurring-check mechanics or sell a manual check as automatic. | Audit proves value without prior fit; setup and integrations create the recurring habit afterward. |
| D3 — audit acquires, check-on-accept retains | Every acquisition result must lead to fit and a concrete recurring integration. | Audit terminal, Markdown, and HTML outputs share the same next-step ladder. |
| D4 — audit optimizes for memorable evidence | Lead with one inspectable catch, then caveats and aggregate context. | Cards and demos show a repository path, rule, evidence, and bounded attribution. |
| D5 — check-on-accept optimizes for speed and restraint | Treat union noise, time-to-brief, and interruption frequency as launch gates. | Combined default-detector data and brief usability evidence exist before automatic rollout. |
| D6 — keep the system model-free in the authoritative path | No generative or opinion-forming model decides findings. | Statistical, graph, inventory, scripted, and local embedding evidence remain replayable and inspectable. |
| D7 — local core remains free | Do not move local check, JSON, SARIF, or portable configuration behind an account or paywall. | Clean install works without identity or cloud. |
| D8 — no default telemetry | Measurement is fixture-based, accepted-history replay, explicit research, or opt-in local records. | No background usage/dismissal upload is introduced. |
| D9 — configuration is portable and user-owned | CLI, hooks, skills, MCP, pre-commit, and CI must honor the same committed rule intent. | The same rule cannot be off in `argot check` but still prompt through a bundled hook. |
| D10 — “voice” is brand, not explanation | Keep compatible command/tool names where renaming would break users, but rewrite explanatory text around repository-grounded evidence. | Hero, README opening, root help, reports, and Action no longer depend on “voice” to explain value. |
| D11–D14 — future gates and honest claims | Separate immediate truth corrections, shipped-capability launches, and later evidence-gated options. | Public copy never outruns released code or measured evidence. |

### Acquisition engine

`argot audit` is already the correct acquisition primitive: it creates a temporary historical worktree, fits at a historical base, checks the base-to-HEAD net diff, attributes surviving findings using concrete commit markers, leaves the working tree untouched, and exits zero when findings exist. Acquisition work should reduce first-run uncertainty, disclose net-window boundaries, improve the proof asset, and turn the result into a setup decision.

### Retention engine

`argot check` is real and evidence-rich but currently invoked manually, by agent choice, by user-wired pre-commit, or in CI. The only bundled automatic lifecycle is a Claude Code pre-write ask for foreign imports. Retention therefore requires: a defined default briefing, combined-noise evidence, one tested automatic post-generation/end-of-turn integration, deduplication, failure degradation, an opt-out, and explicit human ownership of the decision.

### Audit-to-habit funnel

```text
discovery → install → audit → inspect one catch → init/fit → choose integration
          → automatic or clearly user-wired check → act/mute with reason → keep enabled
```

Every arrow needs an explicit next action. The current product has strong individual commands but weak transitions between them.

### Public positioning

Use the canonical four-layer model consistently:

1. **Behavioral truth:** generated code can be valid and still diverge from the repository’s established patterns.
2. **Product job:** Argot surfaces that divergence when the user is deciding whether to accept the change.
3. **Memorable proof:** audit replays recent accepted history and shows a concrete, repository-grounded catch with bounded attribution.
4. **Current tool:** an open-source local CLI with learned and scripted rules, manual/user-wired recurring checks today, and only the lifecycle integrations actually shipped and tested.

### Current-reality constraints and non-negotiables

- Audit is zero **prior Argot setup**, not zero runtime cost or universal success: it needs usable Git history and supported source; semantic analysis may fetch a pinned local model once.
- Audit is a base-to-HEAD net-diff assessment, not commit-by-commit replay and not a census of every transient historical issue.
- “AI-assisted” attribution comes only from recognized markers; “human” means no marker was found; the AI share is a floor.
- A clean check means no configured detector fired on the scanned change, not that the code is correct or fully idiomatic.
- MCP is passive. Its single-hunk scoring tools do not run the full changeset detector composition.
- Skills are invocable instructions, not an automatic lifecycle.
- The pre-commit hook is user-wired commit-time checking. The GitHub Action is CI, not accept time.
- The plugin’s current pre-write hook is Claude-only, fitted-repo-only, foreign-import-only, ask-only, and not an end-of-generation check.
- Local analysis sends no code or usage data and needs no network at all (the embedding model ships inside the binary), but update/version checks, PR review through `gh`, CI artifact downloads, and explicit update operations are network paths.
- Signal quality is a product gate, not a paragraph to add after launch.

## 3. Current experience map

| Stage | Current surface and behavior | Current message | Friction | Strategic problem | Repository evidence |
| --- | --- | --- | --- | --- | --- |
| Discovery | Search/social/GitHub reaches the Astro homepage or README. | “Harness for AI-written code,” “voice,” “statistics, not a second LLM,” “100% local.” | Several concepts compete before the user sees the product job. | Voice/style and AI-review framing obscure awareness at acceptance. | `landing/src/i18n/en.ts`, `fr.ts`; `README.md:1-55`; `landing/public/og.png` |
| Landing page | Homepage order is film, hero, demo, trust, audit, custom rules, engine, proof, setup, CI score, CTA. | Audit is one feature among many; primary CTAs are docs/GitHub. | Acquisition front door is below multiple explanatory sections. | The page does not enact D2–D4. | `landing/src/components/HomePage.astro`; `landing/src/components/{Film,Hero,Demo,Audit,Setup,CiScore}.astro` |
| README | Rich feature catalogue, benchmarks, install paths, plugin, MCP, Action, internals. | “Your codebase has a voice”; audit then fit/check, with broad agent and privacy claims. | Proof, setup, architecture, governance, and every feature compete in one long page. | The README is not a fast audit-first activation surface and exceeds current reality in places. | `README.md` |
| Installation | Shell, PowerShell, npm and release assets are produced; homepage shows npm. | Static binary, platforms, local/no cloud. | Nothing is downloaded after the binary; update behavior and installer-specific update paths are dispersed. | “Zero setup/100% local” is read as “no cost/no egress.” | `dist-workspace.toml`; `README.md`; `landing/src/content/docs/getting-started.md`; `crates/argot-cli/src/update*.rs` |
| First command | Docs and root no-arg help tend toward `argot init && argot check`; homepage install has no strong audit command. | Fit the “voice model” first. | User must trust setup before seeing their own proof. | It reverses the acquisition strategy. | `crates/argot-cli/src/main.rs::print_help_banner`; `landing/src/content/docs/{getting-started,setup,the-commands}.md` |
| Audit | Defaults to 50 first-parent commits, caps 1,000, fits a temporary historical base, checks net base..HEAD, attributes findings, exits 0. | “What AI snuck in,” “before merge,” “all in voice.” | Net-window and first-run-cost boundaries are not prominent. | Strong product proof is weakened by old positioning and incomplete caveats. | `crates/argot-cli/src/audit/*`; `docs/research/evidence/{audit-command,audit-runtime}.md` |
| Audit output | Terminal, JSON v1, Markdown, standalone HTML and share caption exist. | One card plus “Next: argot init … argot check.” | The next action ends at a manual command and offers no integration choice. | Acquisition does not become a habit. | `crates/argot-cli/src/audit/{term,markdown,html,report}.rs` |
| Init/fit | `init` fits, writes default `argot.toml`, updates `.gitignore`, runs health; `fit` writes only `.argot/`. | “Fit today’s voice”; next run `argot check`. | Dirty/branch warnings and suitability notes are useful, but no recurring setup follows. Docs sometimes imply `fit` writes config. | Setup friction remains P1-1 and the handoff remains incomplete. | `crates/argot-cli/src/main.rs::{run_init,fit_repo}`; `crates/argot-engine/src/config.rs`; `landing/src/content/docs/setup.md` |
| Recurring check | `check` covers worktree/staged/unstaged/commit/range and full shipped detector composition. It requires fit artifacts. | “Probabilistic style linter”; clean runs “look clean.” | Cold error says only “run init”; output was not designed/tested as a brief; confidence filtering changes exit behavior despite display-only claims. | Manual execution and overclaiming clean output undermine retention. | `crates/argot-engine/src/check/{orchestrate,render}.rs`; `crates/argot-rules-voice/src/load.rs`; `crates/argot-cli/src/main.rs::CheckCmd` |
| Agent integration | Skills, Claude plugin, MCP, AGENTS instructions, and generic installer are available. | Skills across “70+ agents”; MCP is “proactive voice context.” | Installer reach is conflated with tested automatic support; MCP is passive and partial; skills are agent-chosen. | Users cannot tell what is automatic, passive, tested, or generic. | `skills/`; `.claude-plugin/`; `hooks/hooks.json`; `crates/argot-cli/src/mcp.rs`; `AGENTS.md`; `landing/src/content/docs/agents.md` |
| Claude lifecycle | Bundled `PreToolUse` hook asks on a foreign import before Write/Edit/MultiEdit in fitted repos. | Pre-write guardrail; ask-only. | It is not end-of-generation/accept time and does not honor all portable rule/suppression configuration. | The only automatic lifecycle is narrower than the retention job. | `hooks/hooks.json`; `crates/argot-cli/src/hook.rs` |
| Pre-commit | Manifest runs `argot check --staged`; binary and fit are prerequisites. | CI docs call it informational/non-failing. | Error-severity findings actually exit 1 and block the commit. | Behavior and documentation disagree; default interruption policy is undefined. | `.pre-commit-hooks.yaml`; `landing/src/content/docs/ci.md`; `check/orchestrate.rs::gate_exit_code` |
| CI | Composite Action fits the PR base, runs check, annotates, comments, and optionally gates/publishes a badge. | “Voice check,” “in-voice score.” | Unix archive suffix differs from cargo-dist output; score wording implies conformance. | A likely install defect blocks promotion, and the retained message violates the clean-run boundary. | `action.yml`; `dist-workspace.toml`; `landing/src/content/docs/ci.md` |
| Findings | Human output groups files, rule, confidence, evidence, snippets, stable hashes; JSON/SARIF/GitHub exist. | Rule evidence plus confidence tier. | First lines and multi-rule hierarchy have no accept-time usability evidence. Check JSON says stable but has no schema version. | P1-2 and P2 machine-contract gaps remain. | `crates/argot-engine/src/{finding,output}.rs`; `check/render.rs` |
| Suppression | Excludes, inline ignores, committed mutes, local config, locks, `list-mutes`, `review-mutes`. | Surface, never enforce; reasoned suppression. | The hook can contradict CLI configuration; `.argot/last-check.json` is latest-run mute lookup only. | User-owned intent must be consistent; no durable feedback history exists. | `crates/argot-engine/src/suppress/*`; `argot.toml` parsing; `crates/argot-cli/src/hook.rs` |
| Retention | Background refit keeps models fresh after checks; no automatic accept-time full check is shipped. | Documentation sometimes calls agent-run/commit-time behavior automatic. | User must remember or wire execution. | The product has acquisition proof but no default habit loop. | `crates/argot-cli/src/auto_refit.rs`; plugin/skills/docs surfaces above |

## 4. Surface inventory

| Surface | Repository path | Current role | Main issue | Required change | Strategy reference |
| --- | --- | --- | --- | --- | --- |
| Landing homepage | `landing/src/pages/index.astro`, `fr/index.astro`, `components/HomePage.astro`, `i18n/*.ts` | Main discovery funnel | Voice/AI-harness narrative and feature tour precede audit | Reorder around behavioral truth → audit proof → habit → boundaries | D1–D4, D10 |
| Homepage hero | `landing/src/components/Hero.astro`, `HeroFormula.astro`, `VoiceField.astro` | First comprehension and CTA | Too many old-model claims; docs/GitHub CTAs | State product job without implying shipped automation; primary CTA runs audit | D1, D3, D10 |
| Launch film | `landing/src/components/Film.astro`, `landing/public/film/*` | Visual launch story | Encodes prior positioning and delays proof | Retire, recut, or move below proof after transcript/claim audit | D4, D10 |
| Docs homepage | `landing/src/pages/docs/index.astro`, `content/docs/getting-started.md` | Documentation entry | Starts init/check rather than audit | Make getting started audit-first with explicit branches | D2–D3 |
| README | `README.md` | GitHub discovery and activation | Long voice-led catalogue; stale/overbroad claims | Audit-first opening, bounded integration matrix, canonical evidence links | D3–D4, D10–D12 |
| CLI root help | `crates/argot-cli/src/main.rs::{Cli,print_help_banner}` | Command discovery | Duplicated/drifted; voice linter; init first | One help source, audit first, current-reality tagline | D1–D3, D10 |
| Audit command help | `crates/argot-cli/src/main.rs::AuditCmd` | Acquisition contract | Voice framing; boundaries absent | Explain zero-prior-fit, net-window, attribution and cost limits | D3–D4, D12 |
| Check command help | `crates/argot-cli/src/main.rs::CheckCmd` | Recurring contract | Setup and lifecycle implicit; confidence semantics disagree | State prerequisites, invocation scope, severity vs display behavior | D5, D9 |
| Check human output | `crates/argot-engine/src/check/{render,orchestrate}.rs` | Decision brief | “Style linter,” “looks clean,” untested hierarchy | Brief-first, bounded clean language, evidence retained | D1, D5, D12 |
| Check JSON/SARIF | `crates/argot-engine/src/output.rs` | Machine integration | JSON unversioned; SARIF voice copy | Version/publish JSON contract; reframe descriptions | D9, Product gap P2 |
| First-run errors | `crates/argot-rules-voice/src/load.rs` | Cold-path recovery | Bare `run argot init first` | Offer audit now, then init, then integration | D2–D3, P1-1 |
| Init/fit output | `crates/argot-cli/src/main.rs`, `health.rs` | Fit and suitability | Technical voice model language; manual dead end | Preserve health caveats, add honest habit next action | D3, D5 |
| Audit HTML report | `crates/argot-cli/src/audit/html.rs` | Shareable proof | Style/voice branding; no recurring route; net-window implicit | Behavioral framing, bounded method note, integration CTA | D3–D4, D10 |
| Audit caption/card | `audit/{report,term,markdown}.rs` | Social proof | “All in voice” and AI-centric caption | One concrete catch, floor caveat, no conformance claim | D4, D12 |
| Review command | `crates/argot-cli/src/review.rs`, `main.rs::ReviewCmd` | Local/PR scoring | PR mode’s `gh`/auth/network and fit-basis limits are hidden | Document mode-specific requirements and safe base fit | D9, D12 |
| Voice diff/card/badge | `crates/argot-cli/src/voice_diff.rs`, `action.yml` | PR summary | “100% in-voice” exceeds detector evidence | Preserve command compatibility; present observed findings, not conformity | D10, D12 |
| Installer/update output | `dist-workspace.toml`, `update.rs`, `update_check.rs`, `uninstall.rs` | Distribution and maintenance | Egress and updater eligibility dispersed | Consistent install, offline, model, and update disclosures | D6–D8, D12 |
| Claude plugin | `.claude-plugin/*.json`, `hooks/hooks.json` | Best tested agent bundle | Only pre-write import ask; capability copy broader than lifecycle | Add only measured lifecycle; label every passive/manual part | D1, D5, P0-1 |
| Skills | `skills/*/SKILL.md`, `skills/README.md`, `skills/VERSION` | Cross-agent procedures | Five/six drift; invocable flow can read as automatic | Audit-first setup; honest execution semantics; six-skill parity | D3, D12 |
| MCP | `crates/argot-cli/src/mcp.rs`, `.mcp.json` | Passive agent context | Voice-led; hunk tool is partial, not full check | State passive and detector boundaries; link CLI for full brief | D1, D9–D10 |
| Hooks | `hooks/hooks.json`, `crates/argot-cli/src/hook.rs` | Claude pre-write ask | Not accept time; portable config mismatch | Fix config parity; prototype stop lifecycle behind evidence gates | D1, D5, D9 |
| Pre-commit | `.pre-commit-hooks.yaml`, CI docs | User-wired commit check | Docs call it informational but it gates on errors | Decide and test default; document as commit-time fallback | D5, D12 |
| GitHub Action | `action.yml`, `.github/workflows/*`, CI docs | Recurring PR/CI check | `.tar.xz` vs `.tar.gz`; voice score; not accept time | Fix install, add smoke tests, reframe while retaining non-blocking default | D5, D12 |
| Benchmark page | `landing/src/pages/benchmarks.astro`, `landing/src/data/*`, `benchmarks/*` | Evidence | Figures drift by page and detector vintage | Generate public claims from one versioned manifest | D4–D5, P0-2 |
| Caught in the Wild | `landing/src/pages/caught-in-the-wild.astro`, `fr/*`, data/evidence | Real examples | Sample scope and evidence provenance need visible bounds | Refresh cases, date/source, detector and reproduction details | D4, D12 |
| Social metadata | `landing/src/layouts/Base.astro`, `landing/public/og.png`, favicon/logo | Search/social preview | Old description; JSON-LD lists TypeScript/Python rather than Rust | Update copy/image and correct structured data | D10, D12 |
| Privacy/security | `landing/src/pages/privacy.astro`, `SECURITY.md`, `docs/security/threat-model.md` | Trust boundary | “No network by default” conflicts with model/update paths; background-process wording drifts | Enumerate analysis vs network paths once and reuse | D6–D8, D12 |
| Examples/demo | `docs/demo/*`, homepage demo assets | Product comprehension | Old scenario does not show audit-to-habit or honest integration limit | Produce deterministic audit-first and recurring-flow assets | D3–D4 |
| Contribution docs | `CONTRIBUTING.md`, `crates/README.md`, research/agent docs | Contributor on-ramp | `crates/README.md` describes obsolete monolith/commands | Correct architecture and route public-copy changes through claims data | D9, D12 |
| Agent-facing text export | `landing/src/pages/{llms,llms-full}.txt.ts`, `AGENTS.md` | Machine-readable public docs | Stale language/benchmark counts and voice explanation | Generate from corrected sources and include capability boundaries | D9–D10, D12 |

## 5. Gap assessment

| Known gap | Status and exact evidence | Affected surfaces | Dependencies | Blocking class |
| --- | --- | --- | --- | --- |
| P0-1 — accept-time execution absent | **Confirmed.** Full `argot check` has no automatic acceptance lifecycle. `hooks/hooks.json` wires only Claude `PreToolUse`; skills/MCP are agent-chosen; pre-commit and Action are later/user-wired. | Plugin, hooks, skills, agents docs, landing, README | Lifecycle feasibility, combined-noise gate, briefing design | Full retention-positioning launch blocker |
| P0-2 — combined signal quality unproven | **Confirmed.** `argot-bench` and research evidence are detector-specific; no release-composition union rate or accept-brief exposure metric exists. | Benchmarks, hero proof, reports, integration defaults | Default detector set and accepted-change protocol | Full retention-positioning launch blocker |
| P0-3 — positioning drift | **Confirmed and broader than listed.** Root help, CLI renderers, Action, MCP, metadata, security, `llms.txt`, French strings and contributor docs also drift. | All public surfaces | Claim ledger; current-reality corrections can begin immediately | Immediate honesty blocker |
| P1-1 — fit friction | **Confirmed.** Cold check only says run init; docs often require setup before proof; audit CTA ends at manual check. | CLI errors, getting started, setup skill/docs, README | Audit-to-habit flow and clean-install tests | Activation blocker |
| P1-2 — first briefing lines untested | **Confirmed.** Human renderer is developer-oriented and calls Argot a style linter; no brief comprehension fixtures/research exist. | CLI, hooks, screenshots, demos | Combined default set and brief study | Retention blocker |
| P1-3 — dismissal learning absent | **Confirmed with boundary.** Latest-hit cache supports `mute` only; no durable outcomes. Default telemetry is prohibited. | Local state, benchmark/research workflow | A local-value specification and explicit opt-in research method | Quality improvement, not launch blocker |
| P2 — durable local history | **Absent.** `.argot/last-check.json` is overwritten and contains only visible hits. | CLI/local artifacts/config docs | Evidence that history improves user value; schema/retention design | Later, evidence-gated |
| P2 — versioned machine schema | **Partially present.** Audit JSON has `schema_version = 1`; check JSON and several “stable” JSON commands do not. | Check integrations, docs, CI consumers | Schema inventory and compatibility policy | Integration quality; check JSON is near-term |
| P2 — rule codification | **Real but secondary.** Scripted rules, tests, locks and suggest-rules exist; they should not displace audit/check activation. | Custom-rule docs/skills/landing | Messaging hierarchy only | Quality improvement |
| P2 — broader automatic integrations | **Generic compatibility is real; tested lifecycle support is not.** | Agents docs, plugin/skills claims | Vendor lifecycle matrix and smoke tests | Later after Claude path |
| P3 — platform/governance/cloud | **Gate not crossed.** `rule-tampered` uses an internal `governance` group, but this is not evidence for governance positioning. | README/docs terminology | None; reject public platform work | Rejected now |

### Newly discovered execution gaps

| Gap | Evidence | Consequence |
| --- | --- | --- |
| GitHub Action archive mismatch | `action.yml` downloads `.tar.xz`; `dist-workspace.toml` publishes `.tar.gz`. | Fix and smoke-test before promoting CI as a reliable recurring path. |
| Confidence filter changes gating | `check/orchestrate.rs` computes exit status from `visible` findings after `--min-confidence` filtering, while docs call the option display-only. | Resolve semantics before designing an accept brief or documenting filters. |
| Pre-commit contract contradiction | `.pre-commit-hooks.yaml` directly runs a command that exits 1 on errors; CI docs call the hook informational. | Decide one default and test/document it. |
| Hook ignores portable intent | `hook.rs` loads base detect/scorers but not all rule severity, scope, exclude, mute, or declared-migration decisions. | The same repo can disable a rule in CLI and still be interrupted by the hook. |
| MCP/full-check ambiguity | MCP check/explain score one hunk with the base scorer; full semantic/architecture/integrity/script/tamper composition is absent. | Capability docs must distinguish context from complete changeset checking. |
| Review locality ambiguity | PR-number/URL review shells out to `gh` and may fetch; range/commit review is local. | Privacy and prerequisites need mode-specific wording. |
| Machine-contract drift | Audit is versioned; check and secondary JSON surfaces are not, despite “stable” wording. | Consumers have no explicit compatibility boundary. |
| Benchmark source drift | Foreign, architecture, and integrity figures vary across README, homepage, docs, `llms.txt`, JSON data, and amended research notes. | No public number should be changed by hand; use one generated claim manifest. |
| Security/privacy drift | `SECURITY.md` says no network calls by default while model and passive version paths exist; threat-model background-process language conflicts with detached update/refit work. | Trust claims require one exact, reviewed network/process inventory. |
| Structured-data error | `landing/src/layouts/Base.astro` declares TypeScript/Python as product languages even though the shipped binary is Rust. | Search/social metadata misdescribes the product. |
| Contributor architecture drift | `crates/README.md` still describes the pre-split core and abbreviated command set. | Future agents can plan against obsolete boundaries. |

## 6. Target user journey

1. **Discover.** The hero states the behavioral truth and current product job in seconds. It does not claim an automatic lifecycle that is not released.
2. **Understand.** A compact example shows valid-looking code, the repository evidence that makes it divergent, and the human decision Argot prompts.
3. **Install.** The user chooses a tested installer. The page states supported platforms, offline behavior, and update behavior without implying that code or telemetry is uploaded.
4. **Run audit.** `argot audit` is the primary command. It works without a prior Argot fit and clearly states its history and supported-source prerequisites.
5. **See credible evidence.** The report leads with one inspectable finding and identifies the rule, location, evidence, audited window, net-diff limitation, and marker-based attribution limit.
6. **Understand the next step.** Every audit renderer says: `argot init` creates the current repository fit; then choose one tested recurring workflow. It never says the audit itself installed automation.
7. **Fit the repository.** `init` preserves dirty/branch/suitability warnings, creates portable config, explains generated/data exclusions, and ends with integration choices rather than only a manual check.
8. **Enable recurring checking.** If the measured Claude stop/end-of-turn lifecycle ships, it is the recommended automatic path for Claude. Until then, the recommended path is labeled agent-invoked or user-wired. Pre-commit and CI remain clearly later lifecycle options.
9. **Understand automation limits.** A capability matrix distinguishes automatic lifecycle, passive context, agent-invoked skill, user-wired commit hook, and CI; it names tested versions and detector coverage.
10. **Keep enabled.** The user sees no interruption on clean changes, a concise brief when evidence crosses configured severity, a reasoned mute path, portable configuration honored across surfaces, and no hidden account/telemetry requirement.

### Achievable now versus product requirement

- **Can ship immediately:** remove false/ambiguous claims; put audit first; qualify network, attribution, net-window, MCP, skills, pre-commit, and current automation; fix Action installation; reconcile benchmark sources; improve cold-path guidance.
- **Requires measured implementation:** one automatic full-check lifecycle, deduplication, briefing hierarchy, and the claim that Argot checks automatically near acceptance.
- **Remains evidence-gated:** durable finding history, broader agent lifecycles, explicit local dismissal capture, and every team/cloud/platform option.

## 7. Messaging architecture

| Surface | Primary layer | Recommended direction | Claim boundary |
| --- | --- | --- | --- |
| Landing hero | Behavioral truth + product job | “Generated code can pass normal checks and still break the patterns this repository relies on. Argot surfaces the evidence while you decide whether to accept it.” | Until lifecycle ships, use “designed for”/“helps at” and immediately label current invocation modes; never “automatically checks every acceptance.” |
| Landing supporting line | Current tool | Open-source local CLI; learned and scripted repository evidence; human decides. | Do not say “no model”; distinguish non-generative authoritative analysis from the local embedding model. |
| Hero CTA | Acquisition | Install/run `argot audit`; secondary link to one real report or GitHub. | “Zero prior setup” is allowed with history/model boundary nearby; “zero cost/time” is not. |
| README opening | Behavioral truth → audit | One paragraph, one audit command, one representative output. | Avoid “voice linter,” generic AI reviewer, exclusivity claims, and unbounded accuracy. |
| CLI tagline | Product job | “Surface repository-grounded divergence in proposed changes.” | No automatic lifecycle claim in root help. |
| Audit result | Memorable proof | One finding first; explain why the repository makes it surprising; then bounded window/attribution. | Do not call an empty audit “all in voice” or a complete history census. |
| Check result | Decision brief | “N findings need a look,” prioritized by configured severity/rule; evidence and next action immediately visible. | A clean result means no configured finding on the scan, not correctness/conformance. |
| Docs introduction | Current tool and journey | Audit first, then init, then choose a recurring integration. | Integration labels must be behavioral, not aspirational. |
| Agent integrations | Current capability | Matrix: lifecycle event, automatic/passive/manual, detector coverage, prerequisites, tested versions, fallback. | “70+ agents” may describe installer compatibility only if sourced and must not mean tested automatic support. |
| Benchmark page | Proof | Per-detector facts plus combined default-briefing facts, with data revision, corpus and denominator. | Never generalize one detector’s rate to the whole product. |
| Privacy/security | Current tool boundary | Local analysis, no usage telemetry/account, enumerated network paths, offline mode. | “Nothing leaves” applies to repository content only after exact review; “no network” requires offline mode. |
| Social metadata | Behavioral truth + memorable proof | One specific repository-grounded catch and audit CTA. | No future automatic behavior, stale percentages, or unsupported comparative superlatives. |

### Public language rules

- Prefer: “repository-grounded,” “evidence,” “established pattern,” “proposed change,” “decision brief,” “surface,” “accepted history,” “local analysis.”
- Keep only as secondary brand/compatibility: “voice,” `voice-diff`, `get_voice_context`, `describe-voice`, visual signal motifs.
- Qualify: “zero setup” → “no prior Argot fit”; “local” → local code analysis plus enumerated network operations; “agent support” → installer compatibility versus tested lifecycle; detector percentages → named detector/corpus/revision.
- Remove: “style linter” as product category; “what AI snuck in”; “100% in-voice”; “no other tool”; “can’t hallucinate”; “no model” without scope; generic “AI code review.”

## 8. Information architecture

### Landing page

1. Hero: behavioral truth, current-job wording, audit CTA.
2. One proof: real audit finding and repository evidence.
3. How audit works and its explicit boundaries.
4. From proof to habit: audit → init → best tested recurring workflow.
5. What the released tool catches, grouped by user outcome rather than internal engine count.
6. Capability matrix for Claude, skills, MCP, pre-commit, and CI.
7. Evidence: canonical benchmarks, combined briefing result when available, performance and methodology links.
8. Trust: open source, local analytical path, no account/default telemetry, network/offline details.
9. Install CTA and documentation/GitHub links.

The launch film and statistical formula should not occupy earlier positions unless updated to serve this sequence. Custom rules and the voice-diff badge are secondary depth, not the acquisition story.

### README

Keep it as a fast GitHub on-ramp:

1. one-sentence job and current boundary;
2. install plus `argot audit`;
3. representative proof and method caveat;
4. init plus recurring-integration matrix;
5. concise detector/evidence table;
6. privacy/network summary;
7. canonical benchmark links, limitations, docs and contribution links.

Move detailed scoring, lock mechanics, architecture, benchmark tables, and exhaustive configuration to docs rather than duplicating them.

### Documentation

Target navigation:

- **Start:** Getting started; Audit; Init and fit; Choose a recurring integration.
- **Use:** Check and read the brief; Claude Code; Other agents; MCP; Hooks and pre-commit; GitHub Action.
- **Configure:** Configuration; Rules; Custom rules; Suppressions and locks; Health/freshness.
- **Understand:** What it catches; Limitations; Privacy/network; Architecture/how it works; Benchmarks; Performance; Languages.
- **Help:** Troubleshooting; Real-world scenarios; command reference.

Each concept has one canonical page. `llms.txt`, README summaries, skills, and homepage snippets should link to or generate from those sources rather than restating mutable counts.

### Onboarding and agent docs

Use a decision table instead of one universal setup path. The user selects the environment and sees exactly: automatic event, prerequisites, coverage, failure behavior, opt-out, tested status, and fallback. Claude can be recommended only after its measured lifecycle ships. Generic skills remain the portable manual/agent-invoked path; MCP remains passive context; pre-commit is commit-time; Action is PR/CI.

### Benchmarks and evidence

Keep research evidence under `docs/research/evidence/`. Add one machine-readable public-claim manifest that points to immutable raw/result artifacts, revision, detector composition, corpus, denominator, and allowed wording. Generate the landing page, README snippets where retained, and `llms.txt` facts from this manifest.

## 9. Technical design

### Core and CLI

- Use `argot-core/src/compose.rs` as the shipped-detector composition boundary; combined evaluation must call the same composition rather than reconstruct detectors in a script.
- Resolve confidence filtering so configured rule severity—not display tier—governs exit behavior, and ensure hidden failing findings never produce an unexplained failure.
- Add an explicit additive schema version to check JSON and compatibility snapshots; audit JSON v1 is the precedent. Inventory other JSON before promising stability.
- Design a human-only brief renderer for zero/one/many findings. Preserve stable hashes, rule identifiers, spans, evidence and machine output.
- Keep existing compatibility-sensitive commands/tool names unless a versioned migration is justified. Reframe `voice-diff` output so absence of findings is not full conformance.
- Do not add durable history by default. First specify a local-only user benefit, retention, size, privacy, opt-out/export/delete behavior and schema; implement only if that gate passes.

Affected crates: `argot-engine`, `argot-core`, `argot-cli`, `argot-bench`, and rule crates only when the release-composition harness needs public detector APIs.

### Reports and onboarding

- Treat terminal, Markdown, HTML, JSON and social captions as one audit contract with renderer-specific snapshots.
- Add net-diff and attribution boundaries to human reports without burying the memorable catch.
- End audit and init with the same generated integration choices/capability source to prevent drift.
- Cold check should offer audit for immediate proof and init for recurring use; it must not silently mutate the repository.

### Integrations

- First fix hook/config parity and the Action installer defect.
- Produce a lifecycle capability record from current vendor documentation and tested versions. Existing official surfaces suggest end-of-turn hooks may be feasible for Claude and some compatible plugin hosts, but these are nearest lifecycle proxies, not literal UI acceptance; prototype and measure before shipping or claiming support.
- A candidate automatic hook must debounce multi-edit bursts, run the full changeset composition, remain non-blocking by default, honor config/suppressions, avoid recursive stop loops, degrade silently with a visible setup diagnostic, and never upload code/outcomes.
- Keep pre-write foreign-import ask as a distinct feature; do not relabel it accept time.
- Correct MCP copy and, unless full composition is deliberately added later, keep it a passive context/single-hunk base detector surface.
- Preserve Action input compatibility while replacing “in-voice” conformance language and keeping findings non-blocking by default.

### Landing, README, docs and assets

- The Astro site centralizes homepage copy in `landing/src/i18n/en.ts` and `fr.ts`; every English change needs French parity or an explicit translation gate.
- Update `Base.astro`, JSON-LD, Open Graph image/alt text, sitemap-facing routes, privacy, `llms.txt`, and plain-Markdown doc twins together.
- Keep `VoiceField` only as a visual brand device; it must not force voice into headings or explanations.
- Generate benchmark figures from the claim manifest and fail the site build on unknown/stale revisions.
- Create deterministic demo fixtures and record both cold audit and recurring brief flows; do not hand-edit screenshots that cannot be reproduced.

### Testing and release implications

- Rust: focused unit tests, golden/snapshot render tests, schema compatibility, detector-composition replay, hook config parity and Action/install smoke tests.
- Site/docs: Astro typecheck/build, lint/format, link check, semantic heading/landmark audit, keyboard testing, contrast, reduced motion, 320/768/1280px visual review, English/French parity.
- Journey: clean macOS/Linux/Windows install where runners exist; cold/warm audit timing; init/check; Claude lifecycle if shipped; pre-commit; Action; offline mode; uninstall/update handoff.
- Release code and integration mechanics before dependent claims. Publish an immediate truth-correction release separately if needed. Preserve machine/command compatibility or document versioned migrations.

### Allowed implementation references

- Repository contracts: `AGENTS.md`, `CLAUDE.md`, `crates/argot-core/src/compose.rs`, `crates/argot-engine/src/{detector,finding,output,rules}.rs`, `docs/research/README.md`.
- Existing tests and patterns: `crates/argot-core/tests/*golden.rs`, `check_format.rs`, `rules_gating.rs`, `suppression_roundtrip.rs`; audit module tests; `just verify`; `just landing-check`; `just landing-build`.
- Current lifecycle feasibility must be checked against official vendor documentation at execution time. Starting references: [Claude Code hooks](https://code.claude.com/docs/en/hooks-guide), [VS Code agent hooks](https://code.visualstudio.com/docs/agents/reference/hooks-reference), [VS Code agent plugins](https://code.visualstudio.com/docs/agent-customization/agent-plugins), and each vendor’s current official docs/source for the tested release.

### Anti-patterns to avoid throughout implementation

- Do not write a new detector or tune thresholds to make copy true.
- Do not use one detector’s benchmark as the union/default-briefing rate.
- Do not add a generative judge, default telemetry, account, cloud dependency, or remote code upload.
- Do not hide a finding with copy, confidence filtering, an exclusion, or a mute solely to reduce a reported metric.
- Do not call Stop/end-of-turn “acceptance” without naming the proxy and tested failure modes.
- Do not make the Action, pre-commit, or automatic hook blocking by default merely because the CLI supports exit 1.
- Do not hand-synchronize mutable benchmark numbers across public files.
- Do not let French, `llms.txt`, metadata, security, or contributor docs trail the primary English page.

## 10. Byte-sized task backlog

Tasks are ordered by dependency, not by directory. `L` is intentionally unused. Tasks marked **gated** must be skipped unless their named evidence gate passes.

### Evidence and product decisions

#### EVIDENCE-01 — Freeze the released capability matrix

- **Goal:** Record what every current integration actually triggers, whether it is automatic/passive/invoked/user-wired, its detector coverage, prerequisites, failure behavior, and tested versions.
- **Strategic reason:** P0-1, D1, D12; current support claims conflate installer compatibility with lifecycle execution.
- **Current evidence:** `hooks/hooks.json`, `.claude-plugin/*.json`, `skills/*`, `.pre-commit-hooks.yaml`, `action.yml`, `crates/argot-cli/src/{hook,mcp,review}.rs`, `landing/src/content/docs/agents.md`.
- **Scope:** Add a current-fact integration record under `docs/research/evidence/`; cite current official vendor lifecycle documentation and repository smoke evidence.
- **Out of scope:** Shipping a new hook or promising support based on documentation alone.
- **Dependencies:** None.
- **Complexity:** S.
- **Implementation notes:** Test Claude first. Record Codex, Cursor, VS Code/Copilot and generic skills/MCP separately; “not verified” is an acceptable result.
- **Acceptance criteria:** Each surface has event, automation class, coverage, wiring, tested version/date, known gaps, and safe/forbidden wording.
- **Tests and verification:** Manifest validation plus manual smoke where runnable; archive terminal receipts in the evidence document.
- **Documentation impact:** Source for the later public integration matrix.
- **Public-claim impact:** Requires qualification; enables only claims explicitly marked tested.

#### EVIDENCE-02 — Define the default accept-time briefing policy

- **Goal:** Specify which default rules and severities may appear in an automatic brief and what constitutes an interruption.
- **Strategic reason:** D5 and P0-2; union noise cannot be measured without a fixed release composition and exposure policy.
- **Current evidence:** `crates/argot-engine/src/rules.rs`, `argot-core/src/compose.rs`; defaults mix error and warn rules and confidence is display-only by contract.
- **Scope:** Evidence ADR under `docs/research/evidence/` covering shipped features, warn/error handling, deduplication window, setup errors, and clean behavior.
- **Out of scope:** Tuning detector thresholds, disabling a rule to reach a target, or making the brief blocking.
- **Dependencies:** EVIDENCE-01.
- **Complexity:** XS.
- **Implementation notes:** Measure the actual distribution build. Separate “finding exists,” “brief shown,” and “process exits non-zero.”
- **Acceptance criteria:** One unambiguous policy can parameterize both the benchmark harness and lifecycle prototype.
- **Tests and verification:** Review against `argot rules`, default `argot.toml`, Action defaults and CLI exit semantics.
- **Documentation impact:** Later check/integration docs cite the policy, not internal detector assumptions.
- **Public-claim impact:** No claim enabled by itself.

#### EVIDENCE-03 — Define combined signal-quality and latency gates

- **Goal:** Establish the dataset, denominators, classification method, latency budget, and pass/fail thresholds for an automatic briefing.
- **Strategic reason:** D5, P0-2; noise destroys retention.
- **Current evidence:** Detector-specific harnesses exist in `crates/argot-bench/`, `benchmarks/`, and `docs/research/evidence/`; no full release-composition exposure metric exists.
- **Scope:** Benchmark protocol and fixture manifest; include accepted real changes, true-divergence review, findings/run, briefs/run, per-rule union contribution, clean latency and noisy latency.
- **Out of scope:** Default telemetry, style-based authorship inference, or a single detector’s rate as a proxy.
- **Dependencies:** EVIDENCE-02.
- **Complexity:** S.
- **Implementation notes:** Pin repositories/SHAs and label adjudication uncertainty. Report gating-severity and all-visible results separately.
- **Acceptance criteria:** Another agent can run the protocol without choosing corpora, metrics, or thresholds anew.
- **Tests and verification:** Validate sampling and denominators on a small fixture before the full run.
- **Documentation impact:** Methodology becomes the canonical combined-brief benchmark reference.
- **Public-claim impact:** No automatic claim until BENCH-01 passes it.

#### BENCH-01 — Run the combined default-briefing evaluation

- **Goal:** Measure the actual release detector composition at the intended lifecycle against the EVIDENCE-03 protocol.
- **Strategic reason:** Closes P0-2 and gates retention rollout.
- **Current evidence:** `argot-bench` currently evaluates detector classes separately; `compose.rs` defines the shipped union.
- **Scope:** `crates/argot-bench/`, pinned `benchmarks/` fixtures/results, a new dated evidence record, and machine-readable aggregate data.
- **Out of scope:** Public copy changes, threshold tuning during the run, or excluding unfavorable results.
- **Dependencies:** EVIDENCE-03, CORE-01.
- **Complexity:** M.
- **Implementation notes:** Invoke the production composition. Preserve raw result records and report per-rule marginal contribution plus union exposure.
- **Acceptance criteria:** The report includes corpus/revision, default config, true/false/uncertain classification, findings and briefs per accepted change, latency, and a gate verdict.
- **Tests and verification:** Re-run a deterministic subset in CI; independently recompute aggregate denominators.
- **Documentation impact:** Link from benchmark/limitations docs after claim review.
- **Public-claim impact:** Enables a combined quiet-brief claim only if the predeclared gate passes.

#### UX-01 — Validate the accept-time briefing hierarchy

- **Goal:** Choose the first lines and action hierarchy for zero, one, and mixed-rule briefs.
- **Strategic reason:** P1-2 and D5; a technically accurate report can still be too disruptive.
- **Current evidence:** `check/render.rs` is file-first, says “probabilistic style linter,” and has no comprehension evidence.
- **Scope:** Fixed output prototypes and a short dated research record testing severity, evidence, “why now,” mute/inspect, and human-decision wording.
- **Out of scope:** Changing finding algorithms or machine formats.
- **Dependencies:** EVIDENCE-02; use BENCH-01 distributions when available.
- **Complexity:** S.
- **Implementation notes:** Include clean, one high-actionability hit, many mixed hits, suppressed hits, stale fit, and setup-error cases.
- **Acceptance criteria:** A selected hierarchy and rejected alternatives are documented with reasons and character/line budgets.
- **Tests and verification:** Structured comprehension review with at least fresh and experienced Argot users or, if unavailable, explicit proxy-review limitations.
- **Documentation impact:** Supplies examples for check and integration docs.
- **Public-claim impact:** No direct claim; prerequisite for an automatic workflow.

#### PERF-01 — Measure the clean-install and first-audit matrix

- **Goal:** Establish reproducible cold/warm audit and fit timing across ordinary and large repositories.
- **Strategic reason:** D12; “sixty seconds” and “two minutes” are not generally supported.
- **Current evidence:** `docs/research/evidence/audit-runtime.md` records roughly 25s, 4.7m and 16.9m cold cases; public pages generalize faster cases.
- **Scope:** Pinned repositories, five release targets where practical, cold/warm/offline/failure paths, peak memory.
- **Out of scope:** Performance optimization in the measurement task.
- **Dependencies:** ACTION-01 for released installer validation.
- **Complexity:** M.
- **Implementation notes:** Record hardware and network separately; preserve raw timing logs.
- **Acceptance criteria:** Public speed wording can name a tested range and the large-repo caveat without extrapolation.
- **Tests and verification:** Repeat runs; compare CLI timing phases with wall time.
- **Documentation impact:** Performance, getting-started, audit, landing and README.
- **Public-claim impact:** Changes/qualifies speed and zero-setup claims.

#### BENCH-02 — Establish the canonical public claim manifest

- **Goal:** Give every numeric public claim one versioned machine-readable source and allowed qualifier.
- **Strategic reason:** D4, D12 and P0-3; figures currently drift.
- **Current evidence:** `landing/src/data/{foreign,arch,semantic}.json`, `landing/src/data/benchmarks/latest.json`, and integrity evidence disagree with hard-coded homepage/README/docs numbers.
- **Scope:** A manifest under `landing/src/data/` or `benchmarks/` containing source path, revision, generated date, numerator, denominator, scope, allowed wording and superseded values.
- **Out of scope:** Selecting a favorable number without rerun/provenance or embedding marketing copy in raw results.
- **Dependencies:** None; BENCH-01 entries can be added later.
- **Complexity:** S.
- **Implementation notes:** Resolve foreign 595/605 versus 604/618, architecture 244/252 versus 264/272, and integrity 144/153 versus 155/164 through source lineage, not majority vote.
- **Acceptance criteria:** Foreign visible/masked, semantic, architecture, integrity, languages/corpora, and performance claims have a canonical source or are marked unavailable.
- **Tests and verification:** Schema validation and a script/test that recomputes displayed percentages.
- **Documentation impact:** Becomes source for public docs and generated exports.
- **Public-claim impact:** Enables corrected detector-specific claims; unsupported claims are removed.

#### PROOF-01 — Verify the Caught-in-the-Wild corpus claim

- **Goal:** Determine whether the “33 repositories” and five displayed cases have durable reproducible evidence.
- **Strategic reason:** D4 and D12; memorable proof must be inspectable.
- **Current evidence:** `landing/src/lib/caught-in-the-wild.ts` hard-codes `REPO_COUNT = 33`, has five cases, null upstream URLs and no complete committed run artifact.
- **Scope:** Locate or create a provenance inventory with repo SHA, command/range, finding JSON/hash, adjudication and reconstruction label; otherwise reduce the claim.
- **Out of scope:** Inventing cases or treating authored fixtures as wild catches.
- **Dependencies:** None.
- **Complexity:** S.
- **Implementation notes:** A negative result is valid and should trigger honest copy reduction.
- **Acceptance criteria:** Every retained count/case is reproducible, directly sourced and clearly labels reconstruction; the commit SHA is not displayed as a finding hash.
- **Tests and verification:** Re-run accessible cases; link-check upstream references.
- **Documentation impact:** Caught in the Wild, README and proof methodology.
- **Public-claim impact:** Qualifies or removes the 33-repository claim.

### Product foundation and contracts

#### ACTION-01 — Reconcile GitHub Action release archives

- **Goal:** Make the composite Action download the archive cargo-dist actually publishes.
- **Strategic reason:** A broken recurring integration blocks activation and honest CI promotion.
- **Current evidence:** `action.yml` uses Unix `.tar.xz`; `dist-workspace.toml` specifies `.tar.gz`.
- **Scope:** `action.yml`, ideally deriving suffix/name from one release contract; relevant workflow fixture.
- **Out of scope:** Rewriting Action messaging or changing all distribution formats.
- **Dependencies:** None.
- **Complexity:** XS.
- **Implementation notes:** Verify against a tagged release and `cargo dist plan`; keep checksum verification.
- **Acceptance criteria:** Linux and macOS URLs resolve to published assets and extraction installs the expected version.
- **Tests and verification:** Composite Action smoke on representative Unix runners.
- **Documentation impact:** None unless artifact naming is user-visible.
- **Public-claim impact:** Restores an existing CI capability; does not enable accept-time language.

#### ACTION-02 — Add release-install smoke coverage

- **Goal:** Catch archive, target, checksum and installer drift before publishing an Action or installer claim.
- **Strategic reason:** D12 and release reliability.
- **Current evidence:** No Action self-smoke or clean-install matrix exists; five targets are declared in `dist-workspace.toml`.
- **Scope:** `.github/workflows/`, Action fixtures and release verification scripts using the project’s existing test style.
- **Out of scope:** Expanding supported targets.
- **Dependencies:** ACTION-01.
- **Complexity:** M.
- **Implementation notes:** Cover Linux x64/arm64, macOS x64/arm64 and Windows x64 where runner availability permits; record unavailable emulation explicitly.
- **Acceptance criteria:** Each published target installs, reports the tagged version, and handles a checksum failure safely; Action clean and finding paths run.
- **Tests and verification:** Release-candidate matrix plus post-release URL probe.
- **Documentation impact:** Supported-platform docs cite the tested matrix.
- **Public-claim impact:** Enables precise supported-platform wording.

#### CORE-01 — Make confidence filtering display-only

- **Goal:** Align exit behavior with the documented rule that severity controls gating and confidence controls display.
- **Strategic reason:** D5, D9; hidden filtering must not silently weaken enforcement.
- **Current evidence:** `check/orchestrate.rs` filters to `visible` and passes it to `gate_exit_code`; CLI help and skill/docs describe `--min-confidence` as display-only.
- **Scope:** `crates/argot-engine/src/check/orchestrate.rs`, focused tests, CLI help and affected docs.
- **Out of scope:** Recalibrating confidence or changing default rule severities.
- **Dependencies:** None.
- **Complexity:** S.
- **Implementation notes:** Compute the gate from unsuppressed configured findings, but make any non-zero result intelligible even when weaker hits are hidden.
- **Acceptance criteria:** Changing `--min-confidence` never changes the underlying severity outcome; output explains hidden findings that affect status or the CLI rejects an incoherent combination.
- **Tests and verification:** Matrix over warn/error, `--error-on-warnings`, all tiers, human/JSON/GitHub, suppressed hits.
- **Documentation impact:** Check/reference/skill semantics.
- **Public-claim impact:** No new claim; restores documented behavior.

#### SCHEMA-01 — Version and publish the check JSON contract

- **Goal:** Give `argot check --format json` an additive schema version and an explicit compatibility contract.
- **Strategic reason:** Product gap P2 and D9; integrations need a stable user-owned interface.
- **Current evidence:** `argot-engine/src/output.rs` emits structured JSON without a schema version; audit JSON already uses v1.
- **Scope:** `argot-engine/src/output.rs`, JSON schema file, `check_format.rs`/snapshots, docs.
- **Out of scope:** Redesigning finding semantics or versioning every JSON surface in the same task.
- **Dependencies:** CORE-01.
- **Complexity:** S.
- **Implementation notes:** Prefer top-level `schema_version`; define additive versus breaking changes and unknown-field tolerance.
- **Acceptance criteria:** Schema v1 validates real clean/finding reports; fixtures prove stable required fields; a breaking-change rule is documented.
- **Tests and verification:** JSON Schema validation, golden output, backward-compatible consumer fixture.
- **Documentation impact:** Check/reference/integration docs.
- **Public-claim impact:** Enables “versioned check JSON”; replace vague “stable” until shipped.

#### SCHEMA-02 — Classify secondary machine-readable outputs

- **Goal:** Decide which status/list/inspect/rules/conventions/suggest/voice-diff JSON outputs are public contracts.
- **Strategic reason:** D9 and D12; current “stable” expectations are inconsistent.
- **Current evidence:** Audit is versioned, check is not yet, and several CLI commands emit ad hoc JSON.
- **Scope:** Inventory in a compatibility document; add version tasks only for confirmed public contracts.
- **Out of scope:** Versioning all outputs automatically or renaming `voice-diff` fields without migration design.
- **Dependencies:** SCHEMA-01.
- **Complexity:** XS.
- **Implementation notes:** Record known consumers and compatibility cost before promising support.
- **Acceptance criteria:** Every JSON command is labeled public-versioned, best-effort/internal, or deprecated with rationale.
- **Tests and verification:** Search docs/skills/Action for consumers; sample outputs.
- **Documentation impact:** Command reference.
- **Public-claim impact:** Qualifies machine-readable stability.

#### HOOK-01 — Make the pre-write hook honor portable configuration

- **Goal:** Prevent the bundled pre-write ask from contradicting committed CLI rule intent.
- **Strategic reason:** D9; configuration must remain portable and user-owned.
- **Current evidence:** `crates/argot-cli/src/hook.rs` uses base detect/scorers but does not consistently apply rule off/severity, scopes/excludes, mutes or declared migrations.
- **Scope:** Hook assessment, config loading, path/rule tests, `hooks/hooks.json` timeout.
- **Out of scope:** Adding the accept-time lifecycle or making the hook block.
- **Dependencies:** None.
- **Complexity:** M.
- **Implementation notes:** Some suppression types are diff-hash-specific and may not map to pre-write content; document and test the exact shared subset rather than faking parity.
- **Acceptance criteria:** A disabled/scoped/excluded/migrated foreign-import decision behaves consistently; unfitted/error paths remain no-op and exit success.
- **Tests and verification:** Fixture matrix against equivalent CLI inputs; plugin hook timeout test.
- **Documentation impact:** Plugin/hook/config docs.
- **Public-claim impact:** Strengthens configuration-portability claim, not accept-time coverage.

#### HISTORY-01 — Decide whether durable local finding history is justified

- **Goal:** Make an explicit evidence-gate decision on local finding history and dismissal outcomes.
- **Strategic reason:** P1-3/P2 while preserving D8; this is not a P0 requirement.
- **Current evidence:** `.argot/last-check.json` is overwritten and exists only to resolve current mute hashes; no durable history exists.
- **Scope:** A short design/evidence ADR covering user benefit, storage location, schema, retention, deletion/export, opt-in/out and whether BENCH-01 needs it.
- **Out of scope:** Telemetry, organization aggregation, dashboards, or implementation before the gate.
- **Dependencies:** BENCH-01 and initial lifecycle pilot evidence.
- **Complexity:** S.
- **Implementation notes:** Prefer explicit local research export or existing accepted-history replay if it answers the question without product state.
- **Acceptance criteria:** Decision is “reject/defer” or a bounded local-only specification with measurable user value.
- **Tests and verification:** Privacy/threat-model review and storage-size estimate.
- **Documentation impact:** Limitations/privacy only if approved.
- **Public-claim impact:** None.

#### HISTORY-02 — Implement bounded local finding history (**gated**)

- **Goal:** Add the minimal local record approved by HISTORY-01 without changing mute lookup semantics.
- **Strategic reason:** Only serves P1-3/P2 if the local user benefit was demonstrated.
- **Current evidence:** Depends entirely on the approved HISTORY-01 specification.
- **Scope:** Local gitignored artifact, append/retention logic, inspection/delete/export command only as specified, tests and docs.
- **Out of scope:** Upload, accounts, team aggregation, background telemetry, indefinite retention, or using history as an enforcement score.
- **Dependencies:** HISTORY-01 must explicitly pass; SCHEMA-01 conventions.
- **Complexity:** M.
- **Implementation notes:** Keep `.argot/last-check.json` for current hash resolution unless migration is deliberately specified.
- **Acceptance criteria:** Local records honor retention and deletion, redact nothing by uploading nothing, survive corrupt/truncated tails safely, and are disabled/deferred if the gate did not pass.
- **Tests and verification:** Unit/schema/migration/storage-limit/privacy tests.
- **Documentation impact:** Configuration, privacy, troubleshooting and changelog.
- **Public-claim impact:** No claim beyond “optional local history” if shipped.

#### CLI-01 — Implement the validated human decision brief

- **Goal:** Make `argot check` immediately useful at an automatic lifecycle while preserving evidence depth.
- **Strategic reason:** D1, D5 and P1-2.
- **Current evidence:** `check/render.rs` uses style-linter language and file-first hierarchy; UX-01 selects the replacement.
- **Scope:** Human renderer and golden fixtures; zero/one/many output; clean wording; evidence/hash/verbose affordances.
- **Out of scope:** Machine schema changes, detector scoring, automatic hook wiring.
- **Dependencies:** CORE-01, UX-01; final default behavior informed by BENCH-01.
- **Complexity:** M.
- **Implementation notes:** Keep advisory contract prominent but concise. Replace “looks clean” with scan-bounded language.
- **Acceptance criteria:** Snapshot cases match UX-01; first screen identifies actionability and evidence; no output calls Argot a style linter or implies correctness.
- **Tests and verification:** Golden/snapshot tests, TTY/no-color, narrow terminal, verbose/truncation, suppression/stale-fit notes.
- **Documentation impact:** Check/read-output/examples/screenshots.
- **Public-claim impact:** Changes an existing product description; no automation claim.

### Activation and onboarding

#### CLI-02 — Make root help audit-first and single-sourced

- **Goal:** Remove duplicated root help and make audit the recommended first command.
- **Strategic reason:** D2–D3, D10 and P1-1.
- **Current evidence:** `main.rs` has clap descriptions plus `print_help_banner`; the latter omits commands and says `argot init && argot check`.
- **Scope:** `crates/argot-cli/src/main.rs`, root/no-arg help snapshots.
- **Out of scope:** Renaming compatibility-sensitive commands.
- **Dependencies:** COPY-01 message vocabulary, but an honest audit-first correction may ship earlier.
- **Complexity:** S.
- **Implementation notes:** Prefer clap as the authoritative command registry; group acquisition, daily use and advanced/reference commands if supported cleanly.
- **Acceptance criteria:** Root and no-arg help agree, include all public commands, lead with audit, and state current product job without “voice linter.”
- **Tests and verification:** CLI snapshot of root and all command listings.
- **Documentation impact:** README/getting-started examples.
- **Public-claim impact:** Changes existing positioning; safe current-reality correction.

#### CLI-03 — Turn the unfitted check error into a journey fork

- **Goal:** Help a cold user get proof now or configure recurring use without confusion.
- **Strategic reason:** P1-1 and D2–D3.
- **Current evidence:** `argot-rules-voice/src/load.rs` only reports missing file plus “run argot init first.”
- **Scope:** Error type/message, exit-2 snapshots and relevant docs.
- **Out of scope:** Automatically running audit/init or mutating config.
- **Dependencies:** CLI-02.
- **Complexity:** XS.
- **Implementation notes:** Offer `argot audit` for zero-prior-fit proof and `argot init` for current-repo recurring checks; keep machine stderr predictable.
- **Acceptance criteria:** Missing fit artifacts produce one concise explanation and two distinct next actions; malformed/old artifacts retain specific remediation.
- **Tests and verification:** Missing baseline/config, old version, unsupported source and machine-format cases.
- **Documentation impact:** Troubleshooting/getting started.
- **Public-claim impact:** No new claim.

#### CLI-04 — Add a recurring-integration next step to init/fit

- **Goal:** End successful setup with a tested path to habit rather than only `argot check`.
- **Strategic reason:** D3 and P1-1.
- **Current evidence:** `run_init` reports health and next manual check; no integration chooser follows.
- **Scope:** Init success output and shared integration guidance source; preserve fit’s artifact-only contract.
- **Out of scope:** Claiming automatic behavior or changing `fit` to write `argot.toml`.
- **Dependencies:** EVIDENCE-01; PLUGIN-02 only if making the shipped automatic path recommended.
- **Complexity:** S.
- **Implementation notes:** Before PLUGIN-02, label Claude pre-write, skills, pre-commit and CI accurately. Keep “Not recommended” health notes visible.
- **Acceptance criteria:** Successful init ends with manual smoke check plus environment-specific documentation link/command; no unavailable integration is recommended.
- **Tests and verification:** Init output snapshots for Ready/Ready-with-notes/Not-recommended and offline cases.
- **Documentation impact:** Setup/getting-started/skills.
- **Public-claim impact:** Qualifies current integration behavior.

#### AUDIT-01 — Add audit method and attribution boundaries to every renderer

- **Goal:** Make audit credible without weakening its memorable proof.
- **Strategic reason:** D4 and D12.
- **Current evidence:** Audit is net base..HEAD, not commit replay; attribution is marker-based and a floor; current help/card do not consistently state the net-window boundary.
- **Scope:** `audit/{term,markdown,html,report}.rs`, help, JSON field/docs if needed, renderer snapshots.
- **Out of scope:** Changing the audit algorithm or authorship inference.
- **Dependencies:** None.
- **Complexity:** S.
- **Implementation notes:** Say “patterns present in the audited base-to-head change” and preserve “human means no marker found.”
- **Acceptance criteria:** Terminal/Markdown/HTML/help agree; JSON method metadata is explicit without a breaking change.
- **Tests and verification:** Cross-renderer consistency snapshots and a transient-added-then-removed fixture.
- **Documentation impact:** Audit guide, README, landing.
- **Public-claim impact:** Qualifies zero-setup audit and attribution claims.

#### AUDIT-02 — Reframe audit output and share language

- **Goal:** Replace voice/style/“AI snuck in” copy with behavioral truth and repository evidence.
- **Strategic reason:** D4, D10, P0-3.
- **Current evidence:** Audit terminal/HTML/Markdown use “all in voice,” “repo’s language,” “own style,” and “who wrote it” shorthand.
- **Scope:** Audit renderers, share caption, brand/footer, title/empty state and snapshots.
- **Out of scope:** Full landing/README rewrite or changing findings.
- **Dependencies:** AUDIT-01, COPY-01.
- **Complexity:** S.
- **Implementation notes:** Empty state must say no configured findings in the scanned window, not conformity.
- **Acceptance criteria:** All renderer phrases conform to the claim ledger and lead with one concrete repository-grounded fact.
- **Tests and verification:** Snapshot phrase audit and social-card wrap tests.
- **Documentation impact:** Audit examples and demos.
- **Public-claim impact:** Changes existing audit positioning.

#### AUDIT-03 — Add audit-to-habit next actions

- **Goal:** Make every audit result lead to fit and the best available recurring workflow.
- **Strategic reason:** D3 and P1-1.
- **Current evidence:** Current output stops at `argot init` then manual `argot check`.
- **Scope:** Terminal/Markdown/HTML CTAs and a compact integration link/command; JSON may expose structured next actions additively.
- **Out of scope:** Auto-installing plugins/hooks or claiming the lifecycle is automatic.
- **Dependencies:** AUDIT-01, EVIDENCE-01, CLI-04; PLUGIN-02 only to call that path automatic.
- **Complexity:** S.
- **Implementation notes:** Keep reports useful offline; URLs and commands must be version-stable.
- **Acceptance criteria:** Users can distinguish audit, init and integration; every next action exists and matches current tested behavior.
- **Tests and verification:** Renderer snapshots, link check, manual audit → init → selected path walkthrough.
- **Documentation impact:** All onboarding surfaces.
- **Public-claim impact:** Qualifies current automation; no new automatic claim unless dependency ships.

#### ONBOARD-01 — Make the setup skill audit-first

- **Goal:** Let users see proof before the skill performs repository fitting and exclusions.
- **Strategic reason:** D2–D3; current skill puts audit near the end.
- **Current evidence:** `skills/argot-setup/SKILL.md` performs detailed setup before its audit step; public docs disagree with README’s audit-first order.
- **Scope:** `skills/argot-setup/SKILL.md`, skill metadata/tests and `skills/README.md` flow.
- **Out of scope:** Weakening health/exclusion review or silently editing without user-visible scope.
- **Dependencies:** AUDIT-03, EVIDENCE-01.
- **Complexity:** S.
- **Implementation notes:** Audit is read-only proof; setup remains the later portable configuration workflow.
- **Acceptance criteria:** Skill sequence is install check → audit → interpret → init/exclude/fit → smoke check → recurring integration; current capability limits are explicit.
- **Tests and verification:** Run in a fresh fixture and an existing fitted repo; verify no duplicate plugin/manual hook wiring.
- **Documentation impact:** Setup/getting-started/plugin docs.
- **Public-claim impact:** Changes onboarding, not capability.

#### ONBOARD-02 — Add a reproducible clean-journey fixture

- **Goal:** Exercise install → audit → init → check → integration → finding → reasoned mute → rerun as one product contract.
- **Strategic reason:** The audit-to-habit funnel currently exists only as separate commands/docs.
- **Current evidence:** No full fresh-clone journey test exists; existing demos start from a controlled fitted scenario.
- **Scope:** Test fixture/scripts and receipts under integration/e2e conventions; no production behavior unless a defect is found in a later task.
- **Out of scope:** Testing every agent in one fixture.
- **Dependencies:** CLI-03/04, AUDIT-03, selected integration tasks.
- **Complexity:** M.
- **Implementation notes:** Run offline and normal modes; keep fixture deterministic.
- **Acceptance criteria:** The flow completes from no `.argot` state and documents every mutation/network action and expected exit code.
- **Tests and verification:** Linux CI baseline; platform extensions in QA-02.
- **Documentation impact:** Supplies verified getting-started commands.
- **Public-claim impact:** Supports onboarding claims only.

### Integrations

#### PLUGIN-01 — Prototype a Claude end-of-turn full brief

- **Goal:** Prove whether the current Claude plugin can run one non-blocking full changeset check at the nearest reliable post-generation lifecycle.
- **Strategic reason:** P0-1 and D1; the current pre-write import ask is not the retention engine.
- **Current evidence:** `hooks/hooks.json` uses only `PreToolUse`; official Claude hook documentation exposes Stop/end-of-response behavior, subject to current-version verification.
- **Scope:** Prototype branch/fixture, lifecycle timing/recursion/deduplication receipt and evidence ADR; use full CLI check, not MCP hunk scoring.
- **Out of scope:** Shipping by default, other agents, telemetry, or literal “accept button” claims.
- **Dependencies:** EVIDENCE-01, EVIDENCE-02, CLI-01; BENCH-01 must pass before default rollout.
- **Complexity:** M.
- **Implementation notes:** Test user interrupt, tool bursts, subagents, hook failure, unfitted repo, clean/noisy diff, repeated Stop and background refit interactions.
- **Acceptance criteria:** The prototype records exact lifecycle semantics, average/p95 latency, briefs per user turn, false-repeat rate, coverage, and a ship/reject decision.
- **Tests and verification:** Claude fixture/manual matrix against a pinned released version; confirm no blocking or recursion.
- **Documentation impact:** Evidence only until shipped.
- **Public-claim impact:** No current claim enabled by a prototype.

#### PLUGIN-02 — Ship the measured Claude automatic lifecycle (**gated**)

- **Goal:** Package the PLUGIN-01 lifecycle as an opt-out, non-blocking, full-check brief.
- **Strategic reason:** Closes the first bounded slice of P0-1 and supplies a real retention path.
- **Current evidence:** Requires positive PLUGIN-01 and BENCH-01 verdicts; current plugin otherwise has only pre-write import ask.
- **Scope:** `hooks/hooks.json`, CLI hook/brief adapter if needed, plugin manifest/version, config, dedupe state and integration tests.
- **Out of scope:** Other agents, cloud state, default blocking, new detectors, or calling the proxy literal acceptance.
- **Dependencies:** PLUGIN-01 pass, BENCH-01 pass, HOOK-01, CLI-01, SCHEMA-01 if machine transport is used.
- **Complexity:** M.
- **Implementation notes:** Preserve the distinct pre-write ask; use a separate lifecycle name/config and safe opt-out. Setup errors must not trap the agent in a loop.
- **Acceptance criteria:** Installed plugin automatically runs once per eligible end-of-turn change, stays quiet when clean, presents the validated brief, honors config, never blocks and degrades predictably.
- **Tests and verification:** Plugin E2E clean/noisy/unfitted/error/interrupt/repeat matrix; latency budget and combined-noise canary.
- **Documentation impact:** Plugin, agents, setup, README, landing, changelog.
- **Public-claim impact:** Enables only “automatic end-of-turn checking in the tested Claude Code integration,” not universal accept-time automation.

#### PLUGIN-03 — Add a plugin contract smoke test

- **Goal:** Keep the plugin package, six skills, MCP command and hook manifests internally consistent.
- **Strategic reason:** D9/D12; the current five/six skill drift and untested bundle weaken activation.
- **Current evidence:** Six skill directories exist; `skills/README.md` says five; no end-to-end plugin manifest smoke was found.
- **Scope:** `.claude-plugin/`, `hooks/`, `skills/`, MCP startup fixture and CI check.
- **Out of scope:** Testing every skill installer host.
- **Dependencies:** HOOK-01; include PLUGIN-02 lifecycle only if shipped.
- **Complexity:** S.
- **Implementation notes:** Validate versions, paths, duplicate-hook behavior and binary prerequisite.
- **Acceptance criteria:** CI proves all declared skills exist, MCP starts, hooks parse, unfitted hooks no-op, fitted import ask works, and packaged version fields agree.
- **Tests and verification:** JSON/schema checks plus fixture plugin install where supported.
- **Documentation impact:** Plugin and skills README.
- **Public-claim impact:** Supports “packaged Claude integration.”

#### PRECOMMIT-01 — Make pre-commit non-blocking by default

- **Goal:** Align shipped behavior with the advisory contract and provide an explicit separate gating recipe.
- **Strategic reason:** D5/D12; noise must not unexpectedly block commits.
- **Current evidence:** `.pre-commit-hooks.yaml` runs `argot check --staged` and therefore exits 1 on error findings; `ci.md` calls it informational.
- **Scope:** Pre-commit entry/wrapper or CLI mode, hook manifest, exit-code tests, docs.
- **Out of scope:** Removing the optional hard-gate path or changing CLI check’s general exit contract.
- **Dependencies:** CORE-01, CLI-01.
- **Complexity:** S.
- **Implementation notes:** Preserve exit 2 for setup/usage visibility while making findings advisory, or document an equally explicit strategy-consistent mechanism.
- **Acceptance criteria:** Default hook reports findings and allows commit; setup failures are visible; an opt-in gating hook/config is tested and separately named.
- **Tests and verification:** Clean, finding, warn-only, unfitted and command-error pre-commit fixtures.
- **Documentation impact:** Hooks/pre-commit, CI, README matrix.
- **Public-claim impact:** Changes an existing integration behavior; requires migration note.

#### MCP-01 — Correct MCP coverage and passivity descriptions

- **Goal:** Make agent and user instructions distinguish MCP hunk context from the full CLI changeset check.
- **Strategic reason:** D1, D9, D10 and P0-3.
- **Current evidence:** `mcp.rs` exposes five passive tools; check/explain use base `RepoScorers` and omit semantic, architecture, integrity, script and tamper passes.
- **Scope:** Tool descriptions, startup instructions, `.mcp.json`/plugin copy and docs; retain stable tool names.
- **Out of scope:** Expanding MCP detector composition.
- **Dependencies:** COPY-01.
- **Complexity:** XS.
- **Implementation notes:** Describe `get_voice_context` as repository context and `check_changeset`/CLI/skill as complete changeset checking.
- **Acceptance criteria:** No MCP text implies guaranteed invocation or full detector coverage; passive/user-wired status is explicit.
- **Tests and verification:** MCP protocol snapshot and repository-wide phrase search.
- **Documentation impact:** MCP/agents/plugin/llms.
- **Public-claim impact:** Qualifies an existing claim.

#### INTEGRATION-01 — Publish the tested integration chooser

- **Goal:** Give users one authoritative matrix for choosing Claude plugin, skills, MCP, pre-commit, Action or manual CLI.
- **Strategic reason:** D3, D12 and P1-1.
- **Current evidence:** Capabilities are scattered across `agents.md`, `plugin.md`, `ci.md`, skills and README; “70+” blurs categories.
- **Scope:** One structured source consumed by docs and summarized elsewhere; fields from EVIDENCE-01.
- **Out of scope:** Claiming unsupported agents or repeating full setup steps in every surface.
- **Dependencies:** EVIDENCE-01, PRECOMMIT-01, ACTION-02; PLUGIN-02 status reflected, not assumed.
- **Complexity:** S.
- **Implementation notes:** Include tested date/version and fallback. Treat generic installer compatibility as its own class.
- **Acceptance criteria:** Every listed route has lifecycle, automatic/passive/invoked/user-wired label, prerequisites, coverage, blocking default, and canonical guide.
- **Tests and verification:** Data/schema validation and generated-link tests.
- **Documentation impact:** Central source for landing/README/docs/audit/init.
- **Public-claim impact:** Enables precise supported-integration claims.

#### INTEGRATION-02 — Prototype one additional agent lifecycle (**gated**)

- **Goal:** Test the highest-confidence non-Claude end-of-turn lifecycle without broadening claims prematurely.
- **Strategic reason:** P2 broader integrations only after the first retention path and evidence gates.
- **Current evidence:** Current vendor docs suggest compatible hook formats/events in some hosts, but repository support is untested.
- **Scope:** Select one host from EVIDENCE-01, prototype install/event/input/coverage/failure behavior, record evidence.
- **Out of scope:** A multi-agent abstraction, “70+ automatic,” or shipping more than one unproven host.
- **Dependencies:** PLUGIN-02 shipped successfully, post-release retention/noise review, EVIDENCE-01 positive feasibility for the host.
- **Complexity:** M.
- **Implementation notes:** Reuse existing plugin format only if the released host proves compatibility; account for matcher/input differences.
- **Acceptance criteria:** Explicit ship/reject result with pinned version, trigger semantics, E2E receipt and no unsupported public copy.
- **Tests and verification:** Same clean/noisy/unfitted/repeat/interrupt matrix as Claude.
- **Documentation impact:** Evidence and, only if shipped, other-agents guide.
- **Public-claim impact:** Later enables one named tested integration.

#### ACTION-03 — Reframe Action output without breaking inputs

- **Goal:** Replace voice-score/conformance messaging with an observed findings summary and advisory review language.
- **Strategic reason:** D10/D12; “100% in-voice” exceeds a clean check’s meaning.
- **Current evidence:** `action.yml` name, summary, sticky marker, card and badge depend on `voice-diff`; `voice_diff.rs` smooths heterogeneous hits into a percentage.
- **Scope:** Human/Markdown/Action descriptions and compatibility plan for `voice-diff`; preserve current inputs unless versioned.
- **Out of scope:** Claiming correctness, changing detector scores, or silently breaking badge consumers.
- **Dependencies:** ACTION-02, SCHEMA-02, COPY-01.
- **Complexity:** M.
- **Implementation notes:** Prefer counts/rules/scan scope and “no configured findings” over an “in-voice” percentage. If a badge remains, define its narrow metric visibly.
- **Acceptance criteria:** Clean PRs do not claim full conformance; Action remains non-blocking by default and reports how to opt into gating.
- **Tests and verification:** Action summary/comment/badge snapshots for clean, warn, error and failure paths.
- **Documentation impact:** CI, README, landing and migration note.
- **Public-claim impact:** Changes an existing claim; may deprecate “voice score.”

#### SKILL-01 — Reconcile all skill contracts

- **Goal:** Make the six shipped skills accurate, audit-first where relevant, and consistent with CLI/integration behavior.
- **Strategic reason:** D3, D9, D12.
- **Current evidence:** `skills/README.md` says five; check/setup wording can imply commit-time automation; rule docs must follow live binary behavior.
- **Scope:** `skills/README.md`, six `SKILL.md` files, version metadata and validation.
- **Out of scope:** Adding a seventh skill or changing product strategy through agent instructions.
- **Dependencies:** ONBOARD-01, CORE-01, INTEGRATION-01, PRECOMMIT-01.
- **Complexity:** S.
- **Implementation notes:** Keep the advisory “surface, don’t enforce” contract and exact rule branching; state when a skill is invoked rather than automatic.
- **Acceptance criteria:** Count, commands, rules, severities, prerequisites, integration classes and links match the released binary/docs.
- **Tests and verification:** Skill lint plus command/help checks and plugin manifest cross-check.
- **Documentation impact:** Skills/plugin docs and AGENTS references.
- **Public-claim impact:** Qualifies agent compatibility.

### Public message source and landing website

#### COPY-01 — Create the maintained public claim dictionary

- **Goal:** Turn this plan’s claim ledger into a small maintained source used during public changes.
- **Strategic reason:** P0-3 and D12; wording currently drifts across code, site and docs.
- **Current evidence:** Conflicts exist in README, `en.ts`/`fr.ts`, CLI renderers, Action, `llms.txt`, privacy/security and docs.
- **Scope:** A concise repository document or structured file containing behavioral truth, current capability boundaries, allowed qualifiers, forbidden forms and numeric-manifest references.
- **Out of scope:** Copying the entire strategy into public docs or making marketing constants a runtime dependency.
- **Dependencies:** BENCH-02, EVIDENCE-01; this execution plan supplies the initial nonnumeric ledger.
- **Complexity:** S.
- **Implementation notes:** Assign one owner/source per mutable fact; use generated data for numbers.
- **Acceptance criteria:** Every claim-ledger row has allowed/forbidden wording and source; later claim audit can evaluate it mechanically where possible.
- **Tests and verification:** Phrase/number consistency test prototype.
- **Documentation impact:** Internal public-copy contract.
- **Public-claim impact:** Governs all changed claims; enables none alone.

#### LANDING-01 — Fix current factual, routing and metadata defects

- **Goal:** Ship truth corrections that do not depend on new product behavior.
- **Strategic reason:** D12 and immediate P0-3 correction.
- **Current evidence:** Integrity warn claim is false; CI “never gate” is false; French docs links 404; hreflang advertises missing translations; JSON-LD says TypeScript/Python; duplicate architecture heading ID exists.
- **Scope:** `i18n/*.ts`, relevant components/pages, `Base.astro`, benchmarks page, route/link helpers and tests.
- **Out of scope:** Full homepage redesign or choosing new benchmark numbers.
- **Dependencies:** COPY-01 for final wording; routing/JSON-LD defects can be fixed immediately.
- **Complexity:** S.
- **Implementation notes:** Test actual localized route existence before prefixing or emitting hreflang.
- **Acceptance criteria:** Severity/default-gate claims are correct; no internal 404s/duplicate IDs; structured data describes the Rust product/analyzer accurately.
- **Tests and verification:** Astro build, route/link/hreflang/schema tests.
- **Documentation impact:** None beyond affected pages.
- **Public-claim impact:** Corrects existing claims.

#### LANDING-02 — Replace the hero and primary CTA

- **Goal:** Explain the behavioral problem and product job in seconds, then ask the user to run audit.
- **Strategic reason:** D1, D3, D10 and P0-3.
- **Current evidence:** `Hero.astro`/`en.ts` lead with AI harness, unwritten-rule linting, voice formula and docs/GitHub CTAs.
- **Scope:** Hero, supporting line, CTA, install chip and above-fold metadata in English.
- **Out of scope:** Claiming automatic accept-time execution, final French translation or changing the core visual identity.
- **Dependencies:** COPY-01, PERF-01 for any time wording.
- **Complexity:** S.
- **Implementation notes:** Keep one idea per screen. Demote `HeroFormula`; `VoiceField` may remain purely visual.
- **Acceptance criteria:** A fresh reader can state behavioral truth, current tool and first command; primary CTA is executable audit onboarding.
- **Tests and verification:** Copy review against claim ledger; desktop/mobile visual snapshot; CTA journey test.
- **Documentation impact:** README/metadata should align later.
- **Public-claim impact:** Changes primary positioning; must carry current automation boundary nearby.

#### LANDING-03 — Add the behavioral-problem example

- **Goal:** Show how valid/tested code can still undermine an established repository behavior.
- **Strategic reason:** Four-layer messaging and D4 memorable proof.
- **Current evidence:** Current demo asks whether code “is yours” and focuses on voice; Trust uses an authored integrity example without a clear label.
- **Scope:** One compact homepage example backed by a reproducible fixture/receipt and clear authored-versus-wild labeling.
- **Out of scope:** Generic AI review prose or an unsupported real-world claim.
- **Dependencies:** COPY-01, PROOF-01 or an explicitly authored fixture.
- **Complexity:** S.
- **Implementation notes:** Prefer the canonical weakened-test/routed-around-check pattern only if evidence is inspectable and detector output is real.
- **Acceptance criteria:** Example shows code, repository evidence, rule, and human decision; it never calls a finding a proven bug.
- **Tests and verification:** Snapshot tied to fixture output; accessibility of code/evidence presentation.
- **Documentation impact:** Link to relevant “what it catches”/proof page.
- **Public-claim impact:** Enables one bounded memorable-proof claim.

#### LANDING-04 — Make audit the acquisition section and proof

- **Goal:** Move audit directly after the problem and show a reproducible current report.
- **Strategic reason:** D2–D4.
- **Current evidence:** Audit is the fourth substantive homepage section and its terminal is hand-authored.
- **Scope:** `HomePage.astro`, Audit component/copy, real snapshot/receipt, method caveats and CTA.
- **Out of scope:** Automatic lifecycle claims or hiding cold/large-repo constraints.
- **Dependencies:** AUDIT-01/02, PROOF-02, PERF-01.
- **Complexity:** S.
- **Implementation notes:** Lead with one catch; put net-window/attribution/model details in concise adjacent disclosure with deeper link.
- **Acceptance criteria:** Audit is the first proof after hero/problem; command, output and next step reproduce from a pinned fixture.
- **Tests and verification:** Visual, link and receipt-drift tests.
- **Documentation impact:** Audit guide link.
- **Public-claim impact:** Promotes a current real capability with qualifications.

#### LANDING-05 — Show the audit-to-habit transition and integration boundaries

- **Goal:** Explain fit and recurring choices without pretending universal automation.
- **Strategic reason:** D3, P0-1 and P1-1.
- **Current evidence:** Setup and CI appear late and separately; “Claude Code, Cursor, 70+” blurs execution classes.
- **Scope:** A compact sequence and generated integration chooser summary; remove/replace old Setup/CI hierarchy as needed.
- **Out of scope:** Full setup instructions or promoting an unshipped prototype.
- **Dependencies:** AUDIT-03, INTEGRATION-01; PLUGIN-02 status reflected exactly.
- **Complexity:** S.
- **Implementation notes:** Label automatic, passive, invoked, commit-time and CI visually and textually, not by color alone.
- **Acceptance criteria:** A user can choose a real recurring path and explain its limits; “70+” is removed or scoped to installer compatibility.
- **Tests and verification:** Copy/schema link tests and task-flow usability check.
- **Documentation impact:** Links to integration chooser and setup guides.
- **Public-claim impact:** Qualifies current support; enables named automatic claim only if shipped.

#### LANDING-06 — Rebuild benchmark and evidence presentation

- **Goal:** Render only canonical detector and combined-brief facts with visible scope/provenance.
- **Strategic reason:** D4–D5, P0-2 and D12.
- **Current evidence:** Homepage values are hand-coded; benchmark page mixes old hard-coded data and dynamic JSON and omits combined-noise absence.
- **Scope:** Homepage proof, `benchmarks.astro`, data imports, methodology/source links, integrity card and combined result/“not measured” state.
- **Out of scope:** Inventing a single overall accuracy score or hiding blind spots.
- **Dependencies:** BENCH-02; BENCH-01 for combined facts.
- **Complexity:** M.
- **Implementation notes:** Show revision/date/corpus/denominator and visible-versus-masked foreign boundary; generate percentages.
- **Acceptance criteria:** No displayed number is hand-entered outside the manifest; current blind spots and detector-specific scope are adjacent.
- **Tests and verification:** Data/schema/build test and text assertion against canonical manifest.
- **Documentation impact:** Benchmark/limitations pages align.
- **Public-claim impact:** Corrects and may enable measured combined claim.

#### LANDING-07 — Reframe installation, privacy and open-source trust

- **Goal:** Make free local core, no account/default telemetry, MIT and exact network behavior easy to understand.
- **Strategic reason:** D6–D8 and strategy constraints.
- **Current evidence:** Site says “No LLM/no cloud/100% local”; open source is visible but free individual local core and no default telemetry are not foregrounded.
- **Scope:** Install/trust/CTA/footer/privacy summaries and links; exact supported executable wording.
- **Out of scope:** Pricing/value capture, account/cloud roadmap or security guarantees.
- **Dependencies:** COPY-01, DOCS-10 privacy source, ACTION-02 platform evidence.
- **Complexity:** S.
- **Implementation notes:** Use “one prebuilt executable; no Python/Node runtime,” not universally static; enumerate model/version downloads and offline mode.
- **Acceptance criteria:** Trust block states local analysis/no repo upload, no account, no default telemetry, local core free, MIT, network exceptions and offline option.
- **Tests and verification:** Claim-ledger and privacy-source consistency checks.
- **Documentation impact:** Privacy/security/getting-started.
- **Public-claim impact:** Enables precise local/open-source/free claims; qualifies 100% local/no-model claims.

#### LANDING-08 — Rebuild social metadata and decide the film’s role

- **Goal:** Ensure search/social/video assets express the current strategy and remain accessible/reproducible.
- **Strategic reason:** D4, D10, D12.
- **Current evidence:** `og.png` says voice linter/no model/no GPU; all locales/pages share it; remote film lacks committed transcript/captions/source/checksum and poster implies safety.
- **Scope:** Base metadata/OG cards (language-neutral or EN/FR), film content audit, transcript/captions/provenance or removal/demotion, sitemap indexing decision.
- **Out of scope:** A new brand strategy or safety guarantee.
- **Dependencies:** LANDING-02, COPY-01, PROOF-03.
- **Complexity:** M.
- **Implementation notes:** If the film cannot be brought into claim/accessibility compliance, remove it from the launch path rather than delay the critical funnel.
- **Acceptance criteria:** OG title/description/image match current positioning; film has captions/transcript/version provenance or is removed; proof-page indexing is explicit.
- **Tests and verification:** Metadata snapshots, social-card visual review, media keyboard/screen-reader checks, sitemap test.
- **Documentation impact:** Asset regeneration instructions.
- **Public-claim impact:** Changes social claims.

#### LANDING-09 — Bring French surfaces to claim and route parity

- **Goal:** Publish a reviewed French version of the stable English funnel without broken links or stale claims.
- **Strategic reason:** D12; derived surfaces cannot trail canonical current reality.
- **Current evidence:** `fr.ts` preserves old positioning/70+ claim and prefixes nonexistent `/fr/docs/` routes.
- **Scope:** `fr.ts`, localized routes/links/metadata/OG policy and parity tests.
- **Out of scope:** Translating all English docs unless separately planned.
- **Dependencies:** LANDING-01/02/04/05/07/08 stable.
- **Complexity:** M.
- **Implementation notes:** Use native review, not literal token substitution; retain canonical claim boundaries.
- **Acceptance criteria:** French home/proof routes have no 404, match capability/numeric sources, and advertise only existing hreflang alternates.
- **Tests and verification:** Locale route crawl, copy diff against structured content keys, visual review.
- **Documentation impact:** None.
- **Public-claim impact:** Mirrors approved claims.

#### LANDING-10 — Complete responsive and accessibility validation

- **Goal:** Make the new funnel usable with keyboard, screen reader, reduced motion, zoom and small screens.
- **Strategic reason:** Public activation includes accessibility and responsive behavior.
- **Current evidence:** No skip link/mobile menu; film modal focus containment is incomplete; no automated axe/Lighthouse/visual suite; reduced-motion and demo tabs are comparatively strong.
- **Scope:** Navigation, focus, landmarks, modal, code/tables, 320/375/768/1440 layouts, 200% zoom, automated checks.
- **Out of scope:** Cosmetic redesign unrelated to the funnel.
- **Dependencies:** LANDING-04–09.
- **Complexity:** M.
- **Implementation notes:** Preserve existing reduced-motion/no-JS behavior; do not use color alone for integration classes/findings.
- **Acceptance criteria:** Skip link, accessible mobile nav, focus trap/restoration/inert modal if retained, captions, no serious axe issues, no horizontal loss at target widths.
- **Tests and verification:** Axe/Lighthouse, keyboard-only, screen-reader spot check, reduced-motion and visual matrix.
- **Documentation impact:** Landing contributor/deployment README if new checks added.
- **Public-claim impact:** No claim impact.

### README

#### README-01 — Replace the opening and badges

- **Goal:** Make the GitHub first screen state the behavioral truth, product job, open-source/local boundary and audit action.
- **Strategic reason:** D1–D4, D10 and P0-3.
- **Current evidence:** `README.md` opens “Your codebase has a voice,” AI harness/can’t-hallucinate language and a 100% local badge.
- **Scope:** Title/subtitle/opening paragraph, badges and above-fold commands.
- **Out of scope:** Exhaustive detector documentation or automatic claims.
- **Dependencies:** COPY-01.
- **Complexity:** S.
- **Implementation notes:** Keep current project name/logo; make the first executable product action `argot audit`.
- **Acceptance criteria:** First screen contains one job, one current-boundary sentence and one audit command; no voice-linter/no-cloud absolute badge remains.
- **Tests and verification:** Markdown render/link/badge check and claim audit.
- **Documentation impact:** Root README only; deeper links canonicalize detail.
- **Public-claim impact:** Changes primary positioning and qualifies locality.

#### README-02 — Build an audit-first quick start

- **Goal:** Turn install → audit → interpret → init → integration into the shortest credible path.
- **Strategic reason:** D2–D3 and P1-1.
- **Current evidence:** README already introduces audit before init, but positioning/features precede it and the transition ends at manual check.
- **Scope:** Install and quick-start sections, representative output, current cost/window caveats and next actions.
- **Out of scope:** Duplicating full Audit/Init/Integration guides.
- **Dependencies:** AUDIT-03, PERF-01, INTEGRATION-01.
- **Complexity:** S.
- **Implementation notes:** Put alternate installers behind a concise table/link.
- **Acceptance criteria:** A fresh user can complete the flow using only README commands and knows which recurring behavior is automatic versus wired.
- **Tests and verification:** Execute commands in ONBOARD-02 fixture; link check.
- **Documentation impact:** Must match getting-started.
- **Public-claim impact:** Promotes current audit and qualifies zero setup/speed.

#### README-03 — Replace integration prose with the capability matrix

- **Goal:** State supported agent/commit/CI routes without conflating them.
- **Strategic reason:** D12 and P0-1/P0-3.
- **Current evidence:** README says Claude Code, Cursor and 70+ agents near setup; MCP/skills/plugin automation boundaries are dispersed.
- **Scope:** Compact matrix generated/sourced from INTEGRATION-01; links to canonical guides.
- **Out of scope:** Per-agent setup steps or unsupported lifecycle claims.
- **Dependencies:** INTEGRATION-01; reflect PLUGIN-02 released status only.
- **Complexity:** S.
- **Implementation notes:** Include manual CLI, skills, MCP, Claude hook, pre-commit and Action.
- **Acceptance criteria:** Every row names execution class, prerequisite, coverage and tested status; “70+” is absent or narrowly qualified.
- **Tests and verification:** Matrix data/link validation and claim audit.
- **Documentation impact:** Agent/integration docs remain canonical.
- **Public-claim impact:** Qualifies supported-agent claims; may enable one named automatic Claude claim.

#### README-04 — Reconcile benchmarks, privacy, platforms and limitations

- **Goal:** Remove stale numbers and absolute technical claims while keeping concise proof.
- **Strategic reason:** D5–D8 and D12.
- **Current evidence:** README has old architecture/integrity data, 11-grammar wording, “nothing leaves,” universal static-binary wording and detector-specific rates presented near product-wide claims.
- **Scope:** Benchmark table, how-it-works/privacy paragraph, platform wording, limitations and links.
- **Out of scope:** Reprinting all methodology or research history.
- **Dependencies:** BENCH-02/03, PERF-01, DOCS-10/11, ACTION-02.
- **Complexity:** S.
- **Implementation notes:** Keep visible foreign rate separately scoped; mention masked/in-vocabulary limits and combined result or absence.
- **Acceptance criteria:** All numbers derive from manifest; network/model/platform wording matches canonical docs; 12-language accounting is correct.
- **Tests and verification:** Claim/number drift test and link check.
- **Documentation impact:** Canonical pages linked.
- **Public-claim impact:** Corrects/qualifies benchmark, privacy, speed and platform claims.

#### README-05 — Refresh proof, contribution and strategy links

- **Goal:** Show reproducible assets and make open-source contribution routes obvious without dumping strategy into the README.
- **Strategic reason:** D4 and open-source constraint.
- **Current evidence:** Demo is controlled but its README location text drifts; wild receipts are incomplete; contribution/strategy links are secondary.
- **Scope:** Demo/screenshot links, proof provenance, limitations, docs, contributing, security and canonical strategy link where appropriate.
- **Out of scope:** Copying strategy decisions or internal research chronology into README.
- **Dependencies:** PROOF-02/03/04, DOCS-12.
- **Complexity:** XS.
- **Implementation notes:** Label authored fixtures versus wild catches.
- **Acceptance criteria:** Every visual has a regeneration/provenance link; contribution/license/security routes are visible; no orphaned/stale asset description remains.
- **Tests and verification:** Asset existence/link check and Markdown render.
- **Documentation impact:** `docs/demo/README.md`, CONTRIBUTING if links move.
- **Public-claim impact:** Supports proof/open-source claims.

### Documentation

#### DOCS-01 — Implement the target documentation navigation

- **Goal:** Replace monolithic topic mixing with Start, Use, Configure, Understand and Help journeys.
- **Strategic reason:** D2–D3 and P1-1; users currently encounter init/check before audit and integrations are hard to compare.
- **Current evidence:** Sixteen pages in `landing/src/content/docs/`; no dedicated audit/init/check/MCP/hooks/privacy/limitations/troubleshooting pages.
- **Scope:** Content frontmatter/orders, `DocsLayout.astro`, route redirects/aliases and sidebar tests.
- **Out of scope:** Rewriting all page bodies in this task.
- **Dependencies:** None; reserve final routes used by later tasks.
- **Complexity:** S.
- **Implementation notes:** Preserve old URLs with redirects where static hosting permits or stable compatibility pages.
- **Acceptance criteria:** Target groups/routes exist, no duplicate canonical topic, and old inbound links resolve.
- **Tests and verification:** Astro build, route map and internal link crawl.
- **Documentation impact:** Documentation IA itself.
- **Public-claim impact:** No claim impact.

#### DOCS-02 — Rewrite Getting Started around audit-first activation

- **Goal:** Make the docs entry flow install → audit → understand → init → choose recurring use.
- **Strategic reason:** D2–D3 and P1-1.
- **Current evidence:** `getting-started.md` begins local setup/CI, then init/check, with audit later.
- **Scope:** Getting Started body, commands, choice points and links to dedicated pages.
- **Out of scope:** Full command reference or every installer detail.
- **Dependencies:** DOCS-01, README-02, INTEGRATION-01.
- **Complexity:** S.
- **Implementation notes:** State prior-fit versus runtime/download requirements precisely.
- **Acceptance criteria:** Fresh-clone commands pass ONBOARD-02 and no step implies automatic behavior beyond the selected integration.
- **Tests and verification:** Command execution, link check, claim audit.
- **Documentation impact:** Becomes docs homepage.
- **Public-claim impact:** Changes onboarding and qualifies zero setup.

#### DOCS-03 — Create the canonical Audit guide

- **Goal:** Explain audit purpose, mechanics, formats, boundaries, timing and next actions in one page.
- **Strategic reason:** D3–D4.
- **Current evidence:** Audit detail is split across command reference, README and research evidence.
- **Scope:** New Audit page plus redirects/links from command reference; examples generated from current renderers.
- **Out of scope:** Research implementation chronology or authorship inference beyond markers.
- **Dependencies:** AUDIT-01/02/03, PERF-01, PROOF-02.
- **Complexity:** S.
- **Implementation notes:** Include default 50/cap 1000, first-parent/base selection, net diff, supported-source/history requirements, exit 0 and all formats.
- **Acceptance criteria:** Page answers what it scans, what it misses, what attribution means, costs/network paths, and how to move to habit.
- **Tests and verification:** CLI help parity, sample output snapshot, links.
- **Documentation impact:** Replaces duplicated audit prose.
- **Public-claim impact:** Canonical audit claim source.

#### DOCS-04 — Separate Init and Fit guidance

- **Goal:** Explain why/when `init` writes portable config and `fit` refreshes only local artifacts.
- **Strategic reason:** P1-1 and D9.
- **Current evidence:** `fit_repo` writes only `.argot/`; config module/docs can imply fit writes `argot.toml`; branch/dirty/health caveats are dispersed.
- **Scope:** Dedicated Init/Fit guide, setup page refactor and internal config comments/doc corrections.
- **Out of scope:** Changing artifact/config behavior.
- **Dependencies:** CLI-04, ONBOARD-01.
- **Complexity:** S.
- **Implementation notes:** Cover default branch, dirty tree, generated/data exclusions, `inspect`, refit freshness and offline model behavior.
- **Acceptance criteria:** No page says fit creates portable config; commands and mutations are explicit; health verdict caveats remain.
- **Tests and verification:** Docs-code assertion/search and fixture walkthrough.
- **Documentation impact:** Setup, health/freshness and command reference.
- **Public-claim impact:** Clarifies setup; no new claim.

#### DOCS-05 — Create the canonical Check and briefing guide

- **Goal:** Document the complete changeset check, output, exit semantics and clean-run boundary.
- **Strategic reason:** D5, D9 and P1-2.
- **Current evidence:** Check behavior is split across commands/reading-output/agents; confidence filtering and “looks clean” semantics drift.
- **Scope:** Dedicated Check guide; refactor reading-output and command reference; JSON/SARIF schema links.
- **Out of scope:** Agent-specific lifecycle installation.
- **Dependencies:** CORE-01, CLI-01, SCHEMA-01.
- **Complexity:** S.
- **Implementation notes:** Cover worktree/staged/unstaged/commit/net range, severity versus confidence, suppression, full detector composition and manual nature.
- **Acceptance criteria:** All examples match released help/output; clean wording and exit codes are exact.
- **Tests and verification:** Doctest-style command snapshots and link/schema validation.
- **Documentation impact:** Commands/reading-output become concise references.
- **Public-claim impact:** Canonical recurring-check boundary.

#### DOCS-06 — Split and verify Claude Code guidance

- **Goal:** Provide one tested Claude path covering binary, plugin, skills, MCP, pre-write ask and optional end-of-turn lifecycle if shipped.
- **Strategic reason:** D1, D3, D12.
- **Current evidence:** `agents.md` and `plugin.md` overlap and can read as broader automatic support.
- **Scope:** Claude guide, plugin page consolidation, duplicate-hook warning, install/update/uninstall and failure modes.
- **Out of scope:** Other agents or universal lifecycle claims.
- **Dependencies:** PLUGIN-03, INTEGRATION-01; PLUGIN-02 status exact.
- **Complexity:** S.
- **Implementation notes:** Put a capability table before setup details and distinguish pre-write from end-of-turn.
- **Acceptance criteria:** Commands work in plugin smoke; prerequisites, coverage, blocking default and opt-out are explicit.
- **Tests and verification:** Plugin journey, links and command/version checks.
- **Documentation impact:** Replaces overlapping plugin sections.
- **Public-claim impact:** Enables precise Claude claim only.

#### DOCS-07 — Create Other agents, MCP and hooks guidance

- **Goal:** Document generic compatibility without implying lifecycle automation.
- **Strategic reason:** D9, D12 and P2 gate discipline.
- **Current evidence:** Skills claim 70+; MCP is passive/partial; Codex/Cursor behavior is not tested in-repo.
- **Scope:** Other Agents page plus focused MCP and Hooks subsections/pages; generated capability matrix.
- **Out of scope:** Advertising untested agents or implementing them.
- **Dependencies:** MCP-01, INTEGRATION-01, SKILL-01; INTEGRATION-02 only if a host ships.
- **Complexity:** S.
- **Implementation notes:** Use labels: tested package, generic installer, passive context, agent-invoked, user-wired lifecycle.
- **Acceptance criteria:** Each named host has evidence/status/date or is described generically; CLI is identified as full check.
- **Tests and verification:** Link/config snippets and current vendor-doc recheck.
- **Documentation impact:** Replaces generic sections in `agents.md`.
- **Public-claim impact:** Qualifies supported-agent claims.

#### DOCS-08 — Reconcile CI and pre-commit guidance

- **Goal:** Make Action, other CI and commit-hook behavior exact and reproducible.
- **Strategic reason:** D5/D12; current pre-commit and Action claims contradict behavior/release config.
- **Current evidence:** `ci.md` calls pre-commit non-failing; Action is non-blocking by default but can gate and its archive path is broken.
- **Scope:** CI page(s), Action inputs/permissions/cache/SARIF/comment, pre-commit install/uninstall and explicit gate recipe.
- **Out of scope:** Calling CI accept time or making it default-blocking.
- **Dependencies:** ACTION-02/03, PRECOMMIT-01.
- **Complexity:** S.
- **Implementation notes:** State base-fit behavior, semantic/offline consequences, fork permissions and network paths.
- **Acceptance criteria:** Every documented example is exercised; defaults and opt-in gating agree with manifests.
- **Tests and verification:** YAML/example smoke, link check and Action snapshots.
- **Documentation impact:** Canonical CI/pre-commit source.
- **Public-claim impact:** Corrects integration claims.

#### DOCS-09 — Consolidate configuration, rules and suppression

- **Goal:** Preserve advanced power while giving users one clear finding-to-decision workflow.
- **Strategic reason:** D9 and human-last-word contract.
- **Current evidence:** Configuration is capable but spread over long pages; locks use an internal governance group that must not become positioning.
- **Scope:** Configure/custom-rules/read-output/reference restructuring; portable/local config, rules, migrations, excludes, inline/mute/review-mutes and lock behavior.
- **Out of scope:** New rule features, muting without reason, broad exclusions, or governance marketing.
- **Dependencies:** CORE-01, HOOK-01, SKILL-01.
- **Complexity:** M.
- **Implementation notes:** Lead with inspect → act or reasoned mute → review/prune; mark `rule-tampered` as current mechanism, not platform direction.
- **Acceptance criteria:** One canonical table matches `argot rules`; all suppression examples work and locked behavior is explicit.
- **Tests and verification:** Config fixtures, command snapshots, custom-rule test example and link checks.
- **Documentation impact:** Configure/custom rules/reading output.
- **Public-claim impact:** No new positioning; clarifies existing capability.

#### DOCS-10 — Reconcile privacy, security, architecture and limitations

- **Goal:** Publish one exact analytical/network/process boundary and one honest limits page.
- **Strategic reason:** D6–D8, D12.
- **Current evidence:** `SECURITY.md` says no default network; privacy lists model/version requests; threat model says no background process despite detached refresh/update; how-it-works overstates all detectors learned from history and old extract pipeline.
- **Scope:** Privacy page, SECURITY, threat model, how-it-works/scoring architecture and new Limitations page.
- **Out of scope:** Changing the analytical path, adding cloud, or hiding network behavior.
- **Dependencies:** COPY-01, PERF-01, HISTORY-01 decision.
- **Complexity:** M.
- **Implementation notes:** Enumerate local code analysis, pinned encoder download, version GET, PR review/CI/update network and `ARGOT_OFFLINE=1`; correct current Rust crate/pipeline architecture.
- **Acceptance criteria:** All four surfaces share the same network inventory; no-model/no-background absolute remains; limitations cover masked/in-vocabulary misses, fit suitability and probabilistic findings.
- **Tests and verification:** Repository claim search, architecture review against `compose.rs` and update/refit code.
- **Documentation impact:** Privacy/security/how it works/limitations.
- **Public-claim impact:** Qualifies privacy/model/determinism claims.

#### DOCS-11 — Reconcile benchmark and performance documentation

- **Goal:** Make What It Catches, scoring, performance and benchmark pages consume canonical evidence.
- **Strategic reason:** D4–D5, D12 and P0-2.
- **Current evidence:** `what-it-catches.md` and `the-scoring-model.md` mix foreign/architecture/integrity generations; performance generalizes small-repo timing.
- **Scope:** Relevant content pages, manifest imports/build generation, methodology/limitations links.
- **Out of scope:** Rewriting historical research evidence or changing benchmark results.
- **Dependencies:** BENCH-01/02/03, PERF-01, LANDING-06.
- **Complexity:** M.
- **Implementation notes:** Detector-specific results remain separate; combined result gets its own methodology and “not measured/failed gate” state.
- **Acceptance criteria:** All public figures/revisions match manifest; no detector-specific conclusion is generalized; time ranges are scoped.
- **Tests and verification:** Claim drift test, percentage recompute, link/build check.
- **Documentation impact:** Benchmarks, scoring, catches, performance.
- **Public-claim impact:** Corrects and may enable measured combined claims.

#### DOCS-12 — Add troubleshooting and repair contributor/agent exports

- **Goal:** Close common activation dead ends and remove stale generated/contributor guidance.
- **Strategic reason:** P1-1, D9, D12.
- **Current evidence:** No dedicated troubleshooting page; `CONTRIBUTING.md` points adapters to obsolete `argot-core` path; `crates/README.md` describes old monolith; `llms.txt` says 11 languages and stale facts.
- **Scope:** Troubleshooting page, `CONTRIBUTING.md`, `crates/README.md`, `llms.txt`/`llms-full`, `AGENTS.md` public sections and language page.
- **Out of scope:** Rewriting internal research logs or strategy.
- **Dependencies:** DOCS-02–11, BENCH-02, SKILL-01.
- **Complexity:** M.
- **Implementation notes:** Cover shallow/no history, unsupported files, Not recommended fit, model/offline, stale fit, Action/plugin/pre-commit errors and updates.
- **Acceptance criteria:** Contributor crate/path/command inventory matches workspace; 12-language count and claims are generated/current; common cold-path errors link here.
- **Tests and verification:** Link/command/path existence, generated export snapshots and claim audit.
- **Documentation impact:** Broad but mechanical reconciliation.
- **Public-claim impact:** Corrects derived agent-facing claims.

### Proof assets and validation

#### BENCH-03 — Generate public claims and fail on drift

- **Goal:** Remove hand-synchronized benchmark values from homepage, README, docs and `llms.txt`.
- **Strategic reason:** D12 and P0-3.
- **Current evidence:** `benchmarks.astro` mixes imported/hard-coded values; workflow updates only part of the data; multiple stale copies exist.
- **Scope:** Build/test generator or structured imports, integrity machine data, CI drift check.
- **Out of scope:** Automatically rewriting prose that needs human qualification.
- **Dependencies:** BENCH-02; BENCH-01 data when available.
- **Complexity:** M.
- **Implementation notes:** Fail on missing revision/source/qualifier rather than retaining the previous value silently.
- **Acceptance criteria:** Changing a canonical number updates/invalidates every public consumer; old known denominators trigger a test failure.
- **Tests and verification:** Seeded manifest mutation test and production Astro build.
- **Documentation impact:** Numeric public surfaces.
- **Public-claim impact:** Enforces supported claims.

#### PROOF-02 — Commit a reproducible audit report and card

- **Goal:** Replace hand-authored audit simulations with a pinned artifact users can reproduce.
- **Strategic reason:** D4 memorable proof.
- **Current evidence:** Homepage audit terminal is authored; no committed sample HTML/card with repo, SHA/window, Argot version and command provenance was found.
- **Scope:** Pinned demo repo/commit reference, command script/receipt, JSON/HTML/card/screenshot assets and regeneration notes.
- **Out of scope:** Passing an authored fixture off as a wild catch.
- **Dependencies:** AUDIT-01/02/03, PERF-01.
- **Complexity:** M.
- **Implementation notes:** Prefer a redistributable/pinned fixture; record if semantic model is included/skipped.
- **Acceptance criteria:** One command regenerates artifacts byte-for-byte or with documented dynamic fields; all labels/caveats match current version.
- **Tests and verification:** CI snapshot/receipt verification and visual inspection.
- **Documentation impact:** Audit guide, landing, README.
- **Public-claim impact:** Enables a bounded memorable audit proof.

#### PROOF-03 — Refresh terminal recordings, screenshots and social cards

- **Goal:** Make every visual show the released CLI journey and current message.
- **Strategic reason:** D3–D4, D12.
- **Current evidence:** `docs/demo` is controlled and reproducible but check-first; `landing/public/demo.gif` is orphaned; film/OG/terminal examples are stale or hand-authored.
- **Scope:** `docs/demo/*`, landing assets/components, README visuals, OG cards and regeneration scripts.
- **Out of scope:** New product behavior or unverified wild stories.
- **Dependencies:** CLI-01–04, AUDIT-02/03, LANDING-08, README-02.
- **Complexity:** M.
- **Implementation notes:** Produce audit-first proof and, if PLUGIN-02 ships, a separate honestly labeled recurring-use recording.
- **Acceptance criteria:** Assets name version/context, have alt text/captions where applicable, and have one documented generator; orphan references are removed.
- **Tests and verification:** Render scripts, file consumers, visual review and accessibility checks.
- **Documentation impact:** Demo README and public pages.
- **Public-claim impact:** Visuals reflect only released claims.

#### PROOF-04 — Publish verified wild-case receipts or reduce the page

- **Goal:** Turn retained Caught-in-the-Wild examples into evidence rather than anecdotes.
- **Strategic reason:** D4/D12.
- **Current evidence:** Five hard-coded cases, no upstream links, incomplete corpus proof and ambiguous hash display.
- **Scope:** Case data schema, receipt files, upstream links, dates, actual finding hash, reproduction/reconstruction labels and page count.
- **Out of scope:** Fabricating private upstream evidence or maintaining the number 33 without artifacts.
- **Dependencies:** PROOF-01.
- **Complexity:** M.
- **Implementation notes:** If licenses/privacy prevent receipts, publish fewer cases with stronger detail.
- **Acceptance criteria:** Every displayed fact is sourceable; unsupported totals disappear; authored test-integrity example is clearly labeled unless a real case is verified.
- **Tests and verification:** Schema/link/reproduction checks and editorial evidence review.
- **Documentation impact:** Proof methodology/limitations.
- **Public-claim impact:** Qualifies real-world proof.

#### QA-01 — Add Rust help, renderer and schema regression coverage

- **Goal:** Prevent CLI product language and machine contracts from drifting after the rewrite.
- **Strategic reason:** D9/D12.
- **Current evidence:** Existing goldens cover much engine behavior, but root/subcommand help, cross-audit renderer claims and public phrase boundaries are incomplete.
- **Scope:** CLI help snapshots, check/audit renderer matrix, JSON Schema validation and banned-phrase allowlist for user-visible Rust strings.
- **Out of scope:** Replacing behavior-focused engine tests with snapshots.
- **Dependencies:** CLI/AUDIT/SCHEMA tasks complete.
- **Complexity:** M.
- **Implementation notes:** Allow internal compatibility names such as command IDs, not explanatory phrases.
- **Acceptance criteria:** Root/all public command help and renderer boundary text are covered; schemas validate; `just verify` passes.
- **Tests and verification:** `just verify`, targeted golden refresh review and argot self-check as advisory.
- **Documentation impact:** Test maintenance notes only.
- **Public-claim impact:** Enforces existing approved wording.

#### QA-02 — Validate clean installs and integrations across platforms

- **Goal:** Prove the released journey rather than only source-level intent.
- **Strategic reason:** Activation and D12.
- **Current evidence:** No five-target clean-install journey, Action smoke or plugin contract test currently exists.
- **Scope:** Installer/npm/update/offline/uninstall, audit/init/check, Action, pre-commit and Claude package across supported runners.
- **Out of scope:** Untested agent ecosystem sweep.
- **Dependencies:** ACTION-02, ONBOARD-02, PLUGIN-03, PRECOMMIT-01.
- **Complexity:** M.
- **Implementation notes:** Separate published target support from runner availability and log network operations.
- **Acceptance criteria:** Supported routes succeed from no prior state, exit behaviors match docs, and failures give actionable guidance.
- **Tests and verification:** CI matrix plus manual macOS/Claude receipt where hosted runner cannot cover interaction.
- **Documentation impact:** Supported-platform/integration matrices.
- **Public-claim impact:** Enables exact tested-platform/integration wording.

#### QA-03 — Add site build, link, locale, accessibility and visual gates

- **Goal:** Verify the entire public experience, not only Astro type correctness.
- **Strategic reason:** Landing/docs are product surfaces.
- **Current evidence:** `bun run check` covers lint/format/typecheck; no link, axe, Lighthouse, route, screenshot or full production-build CI gate was found.
- **Scope:** Landing scripts/workflow for production build, internal/external links, locale/hreflang/sitemap, axe/Lighthouse and representative screenshots.
- **Out of scope:** Pixel-perfect tests for every page.
- **Dependencies:** LANDING-10, DOCS-01–12.
- **Complexity:** M.
- **Implementation notes:** Cover home, docs start, audit, integrations, benchmarks, proof and privacy at key widths/reduced motion.
- **Acceptance criteria:** CI fails on broken canonical routes, serious accessibility issues, missing localized alternatives or material visual overflow.
- **Tests and verification:** `just landing-check`, `just landing-build`, automated suite and recorded manual matrix.
- **Documentation impact:** `landing/README.md` and contributor checks.
- **Public-claim impact:** No claim impact.

#### QA-04 — Run the repository-wide claim and journey audit

- **Goal:** Prove every public surface matches the released tag and canonical claim ledger.
- **Strategic reason:** D12 and P0-3 exit gate.
- **Current evidence:** Voice/local/model/agent/numeric claims currently recur across Rust, Astro, Markdown, manifests, images and generated text.
- **Scope:** Search/classify keep/rewrite/remove/qualify/internal/current/future terms; execute the target journey; inspect images/video captions and generated reports.
- **Out of scope:** Reopening strategy or suppressing intentional internal terms.
- **Dependencies:** All public-copy, proof and QA tasks for the release scope.
- **Complexity:** M.
- **Implementation notes:** Use the claim ledger below; record exceptions with path and reason.
- **Acceptance criteria:** No forbidden current claim remains; every allowed claim points to evidence; unshipped automatic behavior is labeled requirement; all journey links/commands work.
- **Tests and verification:** Repository grep, image OCR/manual review, CLI/site builds, clean user-flow validation.
- **Documentation impact:** Final corrections only.
- **Public-claim impact:** Release gate for all claims.

### Release

#### RELEASE-01 — Define compatibility and rollout boundaries

- **Goal:** Split immediate truth corrections from behavior-dependent activation and full positioning claims.
- **Strategic reason:** D12 and sequencing rule: code before dependent messaging.
- **Current evidence:** Several copy defects can be corrected now; automatic lifecycle and combined-noise evidence cannot be claimed now.
- **Scope:** Release checklist/ADR mapping tasks to an honesty patch, foundation release, integration canary and public repositioning release.
- **Out of scope:** Value capture, platform roadmap or delaying truth corrections until automation.
- **Dependencies:** This plan accepted; SCHEMA-02 and PRECOMMIT-01 identify migration surfaces.
- **Complexity:** XS.
- **Implementation notes:** Preserve command/tool IDs where possible; note pre-commit and metric wording behavior changes.
- **Acceptance criteria:** Every public change names the minimum released dependency; no circular “copy waits for copy” dependency remains.
- **Tests and verification:** Dry-run release checklist against task graph.
- **Documentation impact:** Release process notes.
- **Public-claim impact:** Controls when claims unlock.

#### RELEASE-02 — Prepare changelog, migration and release notes

- **Goal:** Tell existing users exactly what behavior, wording and contracts changed.
- **Strategic reason:** D9/D12 and open-source trust.
- **Current evidence:** Root `CHANGELOG.md` delegates to GitHub Releases; generated notes still need intentional migration content.
- **Scope:** GitHub release-note inputs/template, migration notes for pre-commit, check JSON, human output, Action card/voice-diff and plugin lifecycle/opt-out.
- **Out of scope:** A hand-maintained duplicate per-version changelog.
- **Dependencies:** Final scoped implementation tasks and RELEASE-01.
- **Complexity:** S.
- **Implementation notes:** State unchanged guarantees: local core free, no account/default telemetry, config portable, human last word.
- **Acceptance criteria:** Upgrade instructions cover every compatibility/behavior change and link canonical docs.
- **Tests and verification:** Install previous release, upgrade and follow migration on a fixture.
- **Documentation impact:** GitHub release notes, migration page, `CHANGELOG.md` link if necessary.
- **Public-claim impact:** Announces only shipped changes.

#### RELEASE-03 — Canary, publish and post-release verify

- **Goal:** Roll out the recurring integration cautiously, then publish dependent public surfaces against the exact tag.
- **Strategic reason:** D5/D12; signal quality and distribution failures must be observable before a broad claim.
- **Current evidence:** Automatic lifecycle is absent and Action install likely broken; site version is rebuilt via `release.json` stamp.
- **Scope:** Canary plan, tagged artifacts, plugin/skills/MCP registry versions, Action tag, npm, website, docs, release receipts and post-release claim audit.
- **Out of scope:** Default telemetry or broad agent rollout.
- **Dependencies:** QA-01–04, RELEASE-02; PLUGIN-02 only if its gates passed.
- **Complexity:** M.
- **Implementation notes:** If combined/noise or lifecycle canary fails, ship honest audit/manual positioning and keep automation labeled future.
- **Acceptance criteria:** Artifact/action/plugin/npm/site versions agree; public commands work; canary gate is recorded; dependent automatic wording appears only after success.
- **Tests and verification:** Post-release clean install/Action/plugin/website smoke and claim-ledger scan against tag.
- **Documentation impact:** Release notes and status/current-reality update.
- **Public-claim impact:** Unlocks only the claims whose code/evidence shipped.

## 11. Milestones and exit conditions

### Milestone 0 — Evidence and decisions

**Tasks:** EVIDENCE-01–03, BENCH-02, PROOF-01, PERF-01 protocol/start, RELEASE-01.

Work can run in parallel by evidence domain: lifecycle inventory, briefing policy/measurement protocol, benchmark provenance, proof provenance and release compatibility. EVIDENCE-02 follows the lifecycle inventory; EVIDENCE-03 follows the briefing policy.

**Exit condition:** The released capability matrix, default briefing policy, combined-quality protocol, public numeric manifest, audit-performance method, proof provenance status and rollout boundary are written. Unknowns are explicit. No code-dependent claim has been unlocked.

### Milestone 1 — Product foundation

**Tasks:** ACTION-01/02, CORE-01, SCHEMA-01/02, HOOK-01, BENCH-01, UX-01, CLI-01, HISTORY-01 and HISTORY-02 only if gated in.

ACTION, confidence/schema, hook parity, measurement and brief design can run in parallel once their immediate inputs exist. The human brief follows UX-01; the combined run follows CORE-01 and EVIDENCE-03.

**Exit condition:** The Action installs in smoke tests; confidence/severity semantics are coherent; check JSON is versioned; the current hook honors portable configuration; combined union noise/latency has a recorded verdict; the brief is validated; optional history has an explicit reject/defer/implement decision. If the combined gate fails, the milestone still exits with automatic rollout blocked and a documented reason.

### Milestone 2 — Activation and onboarding

**Tasks:** CLI-02–04, AUDIT-01–03, ONBOARD-01/02.

CLI help/errors and audit method/copy can begin in parallel. Audit next actions and init guidance consume the released integration matrix; clean-journey testing follows them.

**Exit condition:** A fresh user can install, audit, understand one finding and its limits, initialize the repository, run a manual complete check, choose a real recurring path, handle a finding and reasoned mute, with every mutation/network step disclosed.

### Milestone 3 — Integrations

**Tasks:** PLUGIN-01; PLUGIN-02 only if gated; PLUGIN-03; PRECOMMIT-01; MCP-01; INTEGRATION-01; ACTION-03; SKILL-01; INTEGRATION-02 only after the later gate.

Pre-commit, MCP, plugin contract and Action message work can run alongside the Claude prototype. Shipping Claude automatic behavior waits for both the combined-quality and prototype verdicts.

**Exit condition:** Every released integration has a tested behavioral contract. Pre-commit is advisory by default with a separate explicit gate. MCP is described as passive/partial. Claude either has a shipped, measured non-blocking end-of-turn full brief or is plainly documented as pre-write/manual only. No broader agent is claimed without its own receipt.

### Milestone 4 — Public repositioning

**Tasks:** COPY-01, LANDING-01–09, README-01–04, ACTION-03 public output, CLI/Audit copy already completed.

Immediate factual corrections (LANDING-01 portions, privacy/automation qualifications, stale claim removal) may ship before new behavior. Hero/README can express the strategic job now only when the current-tool boundary remains adjacent. Wording that says automatic checking occurs waits for PLUGIN-02’s released tag.

**Exit condition:** Landing, README, CLI, reports, Action, metadata and French surfaces use the four-layer model; audit is the front door; current automation limits are visible; all numbers come from the claim manifest; no unshipped behavior is current-tense.

### Milestone 5 — Documentation and proof

**Tasks:** DOCS-01–12, BENCH-03, PROOF-02–04, PROOF-03 visual refresh, README-05, LANDING-10.

The documentation page bodies can run in parallel after IA and their underlying behavior stabilizes. Proof generation waits for final CLI/report text. Benchmark generation can run independently from prose once BENCH-02 is stable.

**Exit condition:** Canonical docs cover getting started, audit, init/fit, check, integrations, Claude, other agents, MCP, hooks/pre-commit, CI, configuration, rules, suppression, privacy, architecture, benchmarks, performance, troubleshooting and limitations. Every public proof is reproducible or explicitly authored/reconstructed. Site accessibility/responsive validation passes.

### Milestone 6 — Release validation

**Tasks:** QA-01–04, RELEASE-02/03.

**Exit condition:** Rust and site gates pass; clean installs and the full journey pass on the declared matrix; links/locales/accessibility/visuals pass; the repository claim audit has no unexplained exception; migration/release notes are complete; published artifacts and website match the released tag. If lifecycle/noise gates failed, release proceeds only with honest manual/user-wired positioning.

## 12. Dependency graph

`[P]` marks groups that can run in parallel once their parent is complete. `[G]` marks an evidence gate rather than an automatic next task.

```text
EVIDENCE-01
├── EVIDENCE-02
│   ├── EVIDENCE-03
│   │   └── BENCH-01 [G: combined-noise/latency pass]
│   └── UX-01
├── INTEGRATION-01
│   ├── CLI-04
│   ├── AUDIT-03
│   ├── README-03
│   ├── DOCS-06
│   └── DOCS-07
└── PLUGIN-01 [after CLI-01]

BENCH-02
├── COPY-01
│   ├── [P] CLI-02
│   ├── [P] AUDIT-02
│   ├── [P] MCP-01
│   ├── [P] LANDING-02
│   └── [P] README-01
├── BENCH-03
│   ├── LANDING-06
│   ├── README-04
│   ├── DOCS-11
│   └── DOCS-12
└── PROOF-01
    └── PROOF-04

ACTION-01
└── ACTION-02
    ├── INTEGRATION-01
    ├── ACTION-03
    ├── PERF-01
    └── QA-02

CORE-01
├── SCHEMA-01
│   └── SCHEMA-02
├── CLI-01 [also UX-01]
├── PRECOMMIT-01
└── BENCH-01

HOOK-01
├── PLUGIN-03
└── PLUGIN-02 [G]

BENCH-01 pass + PLUGIN-01 pass + CLI-01 + HOOK-01
└── PLUGIN-02 [G: ship Claude automatic end-of-turn brief]
    ├── INTEGRATION-01 update
    ├── PROOF-03 recurring recording
    ├── RELEASE-03 canary
    └── INTEGRATION-02 [G: later, after retention review]

AUDIT-01 + COPY-01
└── AUDIT-02
    ├── AUDIT-03
    └── PROOF-02

CLI-02 + CLI-03 + CLI-04 + AUDIT-03
├── ONBOARD-01
└── ONBOARD-02
    └── QA-02

DOCS-01
├── [P] DOCS-02 (ONBOARD-02)
├── [P] DOCS-03 (AUDIT/PROOF-02)
├── [P] DOCS-04 (CLI-04)
├── [P] DOCS-05 (CLI-01/SCHEMA-01)
├── [P] DOCS-06 (PLUGIN-03)
├── [P] DOCS-07 (MCP-01/SKILL-01)
├── [P] DOCS-08 (ACTION-03/PRECOMMIT-01)
├── [P] DOCS-09 (HOOK-01/SKILL-01)
├── [P] DOCS-10 (PERF-01/HISTORY-01)
└── DOCS-11
    └── DOCS-12

LANDING-01 + COPY-01
├── LANDING-02
│   ├── LANDING-03
│   └── LANDING-04 (PROOF-02/PERF-01)
├── LANDING-05 (INTEGRATION-01/AUDIT-03)
├── LANDING-07 (DOCS-10/ACTION-02)
└── LANDING-06 (BENCH-03/BENCH-01)
    └── LANDING-08
        └── LANDING-09
            └── LANDING-10

README-01
├── README-02 (AUDIT-03/PERF-01)
├── README-03 (INTEGRATION-01)
├── README-04 (BENCH-03/DOCS-10/ACTION-02)
└── README-05 (PROOF-02/03/04/DOCS-12)

All scoped implementation and public tasks
├── [P] QA-01
├── [P] QA-02
├── [P] QA-03
└── QA-04
    └── RELEASE-02
        └── RELEASE-03
```

### Parallel work packets

After Milestone 0, separate agents can safely own:

- Action/release reliability: ACTION-01/02.
- CLI contract: CORE-01, SCHEMA-01/02, CLI-01.
- Audit/onboarding: AUDIT-01/02, then AUDIT-03.
- Hook parity: HOOK-01 and plugin contract fixtures.
- Benchmark provenance: BENCH-02/03 and integrity data.
- Proof provenance: PROOF-01/04.
- Immediate public factual fixes: non-dependent parts of LANDING-01 and privacy/automation qualification.

Ownership must remain file-scoped where these streams meet: generated claim data before consumer copy; CLI renderer before visual assets; integration behavior before docs/landing claims.

## 13. Critical path and priority classes

### Shortest safe path to the full new positioning

```text
EVIDENCE-01
→ EVIDENCE-02
→ EVIDENCE-03
→ CORE-01
→ BENCH-01 (pass)
→ UX-01
→ CLI-01
→ PLUGIN-01 (pass)
→ HOOK-01
→ PLUGIN-02
→ INTEGRATION-01
→ AUDIT-03
→ COPY-01
→ LANDING-02/04/05 + README-01/02/03
→ PROOF-03
→ QA-04
→ RELEASE-03
```

BENCH-02/COPY-01 and ACTION-01/02 can run alongside this chain but must finish before public numeric/CI claims. If BENCH-01 or PLUGIN-01 fails, do not fabricate an alternative automatic claim: ship the honest audit-first/manual-user-wired experience and return the failed gate to evidence work.

### Immediate honesty path

The following does **not** require automatic lifecycle implementation and should not wait for it:

```text
EVIDENCE-01 + BENCH-02 + COPY-01
→ remove/qualify false current claims
→ LANDING-01 + README privacy/automation corrections + MCP-01 + AUDIT-01
→ QA-04 scoped claim audit
→ truth-correction release
```

This path may state the product job as a design intent only when the same surface says recurring checks are manual, agent-invoked, or user-wired today.

### Priority classes

**Launch blockers**

- ACTION-01/02: reliable CI installation.
- EVIDENCE-01 and COPY-01: current capability/claim truth.
- BENCH-02/03: canonical public figures.
- AUDIT-01: audit method/attribution boundaries.
- LANDING-01 and immediate README/MCP/privacy corrections.
- QA-04: no unshipped or unsupported current claims.

**Retention blockers**

- EVIDENCE-02/03 and BENCH-01: acceptable combined signal.
- UX-01 and CLI-01: usable decision brief.
- HOOK-01: portable intent.
- PLUGIN-01/02: one measured automatic lifecycle.
- PRECOMMIT-01 and INTEGRATION-01: trustworthy fallback/choice.

**Quality improvements**

- SCHEMA-02 secondary output classification.
- Full docs splitting after critical onboarding pages.
- Wild-case receipts, film enhancement, complete visual regression and generated public examples.
- Contributor architecture refresh and derived agent exports.

**Later evidence-gated options**

- HISTORY-02 durable local history.
- INTEGRATION-02 broader lifecycle.
- Any organization, cloud, account, dashboard, policy or governance platform work.

## 14. Repository-wide public-claim ledger

Status values: **current** (verified), **qualified** (true only with boundary), **unsupported** (remove until evidence), **future** (requirement, not reality), and **internal-only** (may exist in implementation vocabulary but not positioning).

| Claim | Status | Evidence | Allowed wording | Forbidden wording | Surfaces using it |
| --- | --- | --- | --- | --- | --- |
| Argot’s product job | qualified/current intent | Strategy D1; full automatic lifecycle absent in `hooks/`, skills/MCP/pre-commit/Action inventory | “Surfaces repository-grounded divergence while you decide whether to accept a change”; immediately state current invocation | “Automatically checks every change before you accept it” | Hero, README, CLI tagline, docs intro |
| Zero setup | qualified | `audit/mod.rs` temporary historical fit; audit research evidence | “`argot audit` needs no prior Argot fit/config” | Product-wide “zero setup”; “instant on every repo” | Landing audit, README, audit help/reports |
| Audit attribution | qualified | `audit/attribution.rs`; AI allowlisted markers; floor caveat | “Attributes findings from supported commit markers as ai-assisted/human/unknown; AI share is a floor” | “Detects who wrote the code”; “what AI snuck in” | Audit CLI/reports, landing, README, docs |
| Audit history coverage | qualified | `audit/mod.rs` uses net base..HEAD and default 50/cap 1000 | “Checks patterns present in the audited base-to-head change” | “Replays and checks every commit”; “complete historical census” | Audit help/reports/docs/proof |
| Automatic accept-time checking | future, except named shipped lifecycle after PLUGIN-02 | No full automatic hook today; current hook is pre-write import ask | Before PLUGIN-02: “retention target”; after release: exact named event/agent | Universal/current “automatic before accept” | Landing, README, plugin, agents, CLI next actions |
| Claude pre-write hook | current/qualified | `hooks/hooks.json`, `hook.rs` | “Ask-only pre-write check for a new foreign dependency in fitted Claude repos” | “Full check-on-accept”; “checks every detector” | Plugin/agents/setup docs |
| Skills support | current/qualified | Six `skills/*/SKILL.md`; third-party installer | “Six invocable skills available through the skills installer” | “Automatic commit-time checking across 70+ agents” | README, landing Setup, skills/plugin docs |
| MCP support | current/qualified | `mcp.rs` five passive tools; base hunk scoring | “Passive local repository context and base hunk scoring when the agent calls it” | “Full proactive check”; “guaranteed automatic invocation” | Agents/plugin/README/llms |
| Pre-commit | current/contradictory until PRECOMMIT-01 | `.pre-commit-hooks.yaml` directly gates on error findings | Today: “user-wired staged check that may block”; after change: “advisory by default, explicit gate available” | Current docs’ unqualified “informational and never fails” | CI/hooks docs, README matrix |
| GitHub Action | current after ACTION-01 | `action.yml` behavior; archive mismatch blocks reliability now | “User-wired PR check, non-blocking by default, optional gate” after smoke | “Never a merge gate”; “accept-time” | Marketplace metadata, CI docs, landing |
| Local analysis | current/qualified | Core uses local git/tree-sitter/models; privacy code inventory | “Source/history/findings are analyzed locally and are not uploaded” | Unqualified “nothing leaves your machine” if it implies no network | All trust/privacy surfaces |
| Telemetry | current | No usage telemetry path found; strategy D8 | “No default telemetry” | “No network of any kind” | Landing, README, privacy, security |
| Model usage | qualified | Base statistical scorer; embedded local Jina semantic encoder; no generative judge | “No generative or opinion-forming model in the authoritative analytical path; semantic rules use a pinned local encoder” | “No model,” “No LLM anywhere,” “statistics only” | Hero, engine, README, docs, OG, llms |
| Network behavior | qualified | Passive version GET, review via `gh`, update and CI downloads; offline mode | Enumerate paths; “`ARGOT_OFFLINE=1` disables automatic network access” | “No network of any kind” without explaining version/update behavior | Privacy, SECURITY, threat model, setup |
| Deterministic/replayable | qualified | Pinned artifacts and golden/model tests; external/environment inputs still matter | “Authoritative findings are inspectable and designed to replay for pinned inputs/config” | “One thing that can’t hallucinate”; universal magic guarantee | Landing, README, architecture docs |
| Foreign visible-symbol detection | current/qualified pending BENCH-02 lineage | `foreign.json` 595/605; latest aggregate separates masked cases | “98% (595/605) of visible-symbol foreign fixtures in the named revision” | Product-wide 98%; mixing 604/618 without lineage | README, homepage, What It Catches, llms |
| Architecture detection | disputed until BENCH-02 | Current `arch.json` 264/272, 0/148; public old 244/252, 0/140 | Only canonical manifest wording after lineage decision | Hand-changing one surface to either generation | README, landing, docs, AGENTS, llms |
| Integrity detection | disputed until BENCH-02 | Amended evidence 155/164, 0/106, 45/3602; public old values and denominators | Only canonical manifest with authored/control/accepted-history scopes | Product-wide “94%,” stale 1.12/1.13/1.25 without source | README, homepage, scoring/catches, llms |
| Combined detector noise | unsupported/currently unmeasured | No production-composition acceptance replay | “Not yet measured” until BENCH-01; then exact protocol/revision | Generalizing 0.29% foreign over-fire to the automatic full brief | Positioning proof, benchmarks, integration launch |
| Clean check | qualified | `check/orchestrate.rs`; detector limitations and config | “No configured findings on the scanned change” | “100% in-voice,” “matches the repo,” “correct,” “looks clean” | CLI, voice-diff, Action, badges, audit empty state |
| Speed | qualified | `audit-runtime.md`; small and large cold/warm ranges | Named repo/hardware/cold-warm measurements or bounded ranges | Universal “sixty seconds”/“two minutes” | README, landing, performance docs |
| Supported languages | current but count drift | `argot-lang` adapters/Cargo grammars; 12 ship | “12 tested language adapters” with current table | “11” in llms/acknowledgements; “all languages” | README, docs, llms, benchmark scopes |
| Supported platforms | current/qualified after smoke | Five targets in `dist-workspace.toml`; Windows dynamic UCRT | Name macOS arm/x64, Linux glibc arm/x64, Windows x64; “one prebuilt executable, no Python/Node runtime” | Universal “fully statically linked” | Install, README, landing, docs |
| Open source | current | MIT `LICENSE`, public repository | “MIT-licensed open source” | Enterprise/platform implications | Landing, README, footer |
| Free individual local check | standing decision/current policy | Strategy D7; current distribution has no account/paywall | “The individual local core check remains free” | Future pricing/value-capture promises beyond strategy | Landing trust, README, docs |
| Account/cloud requirement | current negative | No account/server requirement in core | “No account or cloud required for core audit/check” | “No network ever”; future cloud roadmap | Landing, README, privacy |
| Configuration ownership | current/qualified | `argot.toml`, `argot.local.toml`, suppressions/migrations; hook gap until HOOK-01 | “Portable, user-owned config honored by supported surfaces” after parity fix | Claiming parity before HOOK-01 | README, docs, integrations |
| Voice | internal/brand-compatible | Command/tool/artifact names and visual motif | Brand texture, compatibility names, historical/internal model terms | Hero/product category/explanatory crutch; “voice score” as conformity | Repo-wide; rewrite primary public occurrences |
| Governance | internal-only capability term | Rule group `governance`/`rule-tampered`; strategy rejects positioning | Explain the one current lock-protection mechanism where needed | “AI governance platform,” organizational control positioning | Rules/config/README; keep out of hero/roadmap |
| Caught in 33 repositories | unsupported until PROOF-01 | Hard-coded count, five cases, incomplete receipts | Verified count with corpus artifact, or only count displayed verified cases | Retain 33 without evidence | Caught in the Wild, README |

### Term classification summary

- **Keep:** audit, repository-grounded, evidence, local analysis, open source, human decision, configurable rules, accepted history.
- **Rewrite:** voice linter, your codebase has a voice, AI harness, catches AI mistakes, “before merge” without invocation, 100% local, deterministic, no models, 70+ agents.
- **Remove:** generic AI code review, “what AI snuck in,” “100% in-voice,” comparative “no other tool” claims, universal no-network/no-model/no-GPU claims.
- **Qualify:** zero setup, attribution, privacy, speed, platform support, agent support, detector accuracy, semantic/offline behavior.
- **Internal-only:** `voice` group/model artifact terms, `governance` rule group, research era labels, compatibility command/tool IDs.
- **Current-reality claim:** audit, manual/full check, Claude pre-write ask, passive MCP, invocable skills, user-wired pre-commit/Action, local portable config.
- **Future requirement:** full automatic check-on-accept, broader lifecycle support, combined-noise claim until measured, durable local history until justified.

## 15. Rejected work

| Rejected work | Standing decision that rejects it | Why it is not in the backlog |
| --- | --- | --- |
| Reposition Argot as generic AI code review | D1 and D10; canonical behavioral truth/product job | Broad review prose loses repository evidence and acceptance awareness. |
| Rewrite or reopen the strategy | Strategy hierarchy and explicit mission constraint | This plan executes D1–D14; it does not seek a new positioning. |
| Claim universal automatic accept-time checking now | D12; P0-1 current reality | No full lifecycle is shipped. Only a named tested release may unlock bounded wording. |
| Default telemetry or automatic dismissal upload | D8 | Measurement must use local fixtures/accepted-history replay or explicit research; no hidden data collection. |
| Mandatory cloud or account | D7–D8 | The core must remain usable locally without identity or service dependency. |
| Paywall audit/check, JSON, SARIF or portable config | D7 and free-local-core constraint | Value capture is outside strategy; local individual check remains free. |
| Add a generative/opinion-forming judge | D6 | It would alter the trusted authoritative analytical path and evidence contract. |
| Organization dashboard before evidence gate | D11/D13–D14 | No demonstrated individual-to-team demand or sharing behavior justifies it. |
| Governance/enterprise positioning | D11 and Founder manifesto | The `governance` internal rule group is not permission to build/market a platform. |
| High-noise default blocking/gating | D5 and signal-quality constraint | Findings remain prompts; automatic/commit/CI paths are advisory by default unless the user explicitly opts in. |
| Disable rules, widen excludes or auto-mute to improve launch metrics | Argot contract and D5 | This hides evidence instead of improving the product and violates user ownership. |
| Durable history as a launch blocker | Product gaps P2; HISTORY-01 gate | It is implemented only if a bounded local user benefit is demonstrated. |
| Broad multi-agent abstraction before one lifecycle works | P0-1 before P2 broader integrations | First prove and retain one named integration; expand one tested host at a time. |
| Infer AI authorship from code style | Audit attribution contract | Attribution remains concrete-marker-based and a floor. |
| Replace “voice” by another vague metaphor everywhere | D10 | Primary explanation becomes behavioral and evidence-based; compatibility/brand terms can remain where harmless. |
| Publish a single overall accuracy or “repository conformity” score | D5/D12 and detector evidence scopes | Heterogeneous detectors and clean-run limits do not support it. |

## 16. Final validation checklist

### Corpus and repository inspection

- [x] `FOUNDER.md` was read in full first.
- [x] `ARGOT_STRATEGY.md` was read in full and treated as canonical.
- [x] `ARGOT_CURRENT_REALITY.md` was read in full and treated as authoritative current fact.
- [x] `ARGOT_PRODUCT_GAPS.md` was read in full and reconciled gap by gap.
- [x] `ARGOT_STRATEGY_CARD.md`, `ARGOT_STRATEGY_CHANGELOG.md`, `REORGANIZATION_REPORT.md`, and derived `ARGOT_STRATEGY.html` were read in full.
- [x] The Rust workspace, composition root, detector/rule crates, configuration, suppressions, model integration, audit, check, review, MCP, hooks, Action and distribution/update surfaces were inspected.
- [x] The complete landing implementation, routes, components, English/French copy, metadata, assets, styles, build/deployment configuration and static accessibility/responsive behavior were inspected.
- [x] Root `README.md` was read in full.
- [x] All 16 user-facing docs pages, privacy, benchmark, proof, `llms.txt`, contribution, security, agent and demo surfaces were inspected; current claim-bearing research evidence was traced.
- [x] CLI root and every public subcommand help surface were inspected, along with human/JSON/SARIF/GitHub and audit renderers.
- [x] Claude plugin, all six skills, hooks, MCP, AGENTS instructions, pre-commit and GitHub Action behavior were inspected and classified.
- [x] Benchmark sources were traced to JSON/harness/evidence records; conflicts were preserved as decisions rather than silently resolved.
- [x] Public claims were reconciled with current reality in the claim ledger.

### Plan quality and strategy integrity

- [x] Every backlog item has a stable ID, single outcome, strategy reason, current evidence, exact likely scope, out-of-scope boundary, dependencies, sub-L complexity, implementation notes, acceptance criteria, verification, documentation impact and public-claim impact.
- [x] Dependencies and parallel work packets are explicit.
- [x] Launch blockers, retention blockers, quality improvements and later evidence-gated options are separated.
- [x] Milestones have testable exit conditions.
- [x] No future-gated organization/platform/governance work was introduced.
- [x] No unshipped feature is planned to be marketed as current.
- [x] Honest current-reality corrections are allowed to ship before automation.
- [x] Local-first analysis remains intact.
- [x] No-default-telemetry remains intact.
- [x] No generative or opinion-forming model enters the authoritative analytical path.
- [x] Portable, user-owned configuration remains the contract.
- [x] The individual local core check remains free and requires no account/cloud.
- [x] “Voice” remains available as brand/compatibility vocabulary but does not carry the product explanation.
- [x] The plan does not reopen the strategy or introduce value-capture work.

### Execution-time final gate

Before the public repositioning release, the executing team must turn each item below from unchecked to checked in the release record:

- [ ] Combined default-briefing quality and latency pass the predeclared gate, or automatic current-tense wording is absent.
- [ ] The named automatic lifecycle passes its end-to-end and canary tests, or it is documented as future/manual.
- [ ] Action/install artifacts pass the supported-target smoke matrix.
- [ ] Audit, check and machine schema snapshots match the released binary.
- [ ] Homepage, README, docs, CLI, reports, Action, skills, MCP, plugin, metadata, privacy/security, `llms.txt`, French surfaces and visual assets pass the claim ledger.
- [ ] Benchmark figures are generated from the canonical manifest and include detector/revision/denominator qualifiers.
- [ ] Clean-install audit-to-habit and reasoned-suppression journeys pass.
- [ ] Link, route, locale, accessibility, reduced-motion, responsive and production-build checks pass.
- [ ] Release notes describe every behavior/schema/integration migration.
- [ ] The published tag, binaries, npm package, plugin/skills/MCP metadata, Action and website report the same version.
