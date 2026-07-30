# Argot — Current Product Reality

**Purpose.** A factual snapshot of what Argot demonstrably does today, verified against the
repository (code, CLI `--help`, docs, tests, `action.yml`, `dist-workspace.toml`), not against
strategy prose. This document is the evidence base for `ARGOT_STRATEGY.md` and
`ARGOT_PRODUCT_GAPS.md`. When strategy prose and this file disagree, this file wins on matters of
current fact.

**Verification date:** 2026-07-22. **Binary inspected:** `target/release/argot` (built from the
current tree, `argot --help` run directly). Shipped release features (`dist-workspace.toml`):
`["self-update", "semantic", "arch", "integrity", "script"]`.

**Status vocabulary** (exactly one per capability):
- **Exists and verified** — present and confirmed working from code/CLI/docs.
- **Exists with limitations** — present, with a material caveat named.
- **Partially implemented** — some of the claimed behavior exists; a core part does not.
- **Planned but absent** — referenced somewhere but not implemented.
- **Strategic hypothesis** — a belief, not a shipped capability.
- **Unknown / requires manual verification** — not conclusively established here.

**Public-claim key:** Yes · Yes, with qualification · No, not yet.

---

## 1. Reality inventory table

| Capability | Current status | Evidence | Strategic role | Public claim allowed? | Gap / next action |
|---|---|---|---|---|---|
| `argot audit` | Exists and verified | `crates/argot-cli/src/audit/`; `audit/mod.rs` fits a temp worktree at the base commit, user tree untouched; `window.rs` default 50 commits, cap 1000 | Acquisition front door (North Star step 1) | Yes | Verify first-run wall-clock on large repos |
| — zero setup (no prior fit) | Exists and verified | Fits its own temporary model; `the-commands.md` "works on a fresh clone with no setup" | Removes activation friction | Yes | — |
| — AI vs human attribution | Exists with limitations | `audit/attribution.rs`: allowlist of agent emails / bot slugs / commit-footer markers; "human" = "no markers found"; AI share is a floor, not a census | Makes the audit story concrete and honest | Yes, with qualification (say "attributed from commit markers; a floor, not a census") | — |
| — shareable HTML card + caption | Exists and verified | `audit/html.rs` (self-contained, no external requests); `audit/report.rs::share_caption` | Virality of the front door | Yes | — |
| `argot check` | Exists and verified | `main.rs` `run_check_cmd`; scores workdir/ref/range/commit/staged | The retention interaction (manual today) | Yes | See "acceptance-moment auto-run" |
| — requires prior fit | Exists and verified | `crates/argot-rules-voice/src/load.rs` errors exit 2 "run `argot init` first" if no `scorer-config.json` | Setup is a precondition of check | Yes, with qualification | Onboarding must make fit near-automatic |
| Fit / setup (`init`, `fit`) | Exists and verified | `main.rs` `fit_repo` = train + calibrate; `init` adds health report + `.argot/.gitignore` | Precondition for check/review/conventions/mcp | Yes | — |
| Embedded semantic model (no download) | Exists and verified | `crates/argot-rules-semantic/src/static_embedder.rs` `include_bytes!`s a 15.6M-parameter distilled table (int8, 256-d) + its tokenizer from `crates/argot-rules-semantic/model/`; nothing is fetched, cached or checksummed at runtime | Powers `redundant`/`misplaced` only | Yes (analysis needs no network at all) | Weights are Apache-2.0-derived and attributed in `NOTICE`; the ~17.5 MB in git is disclosed in `CONTRIBUTING.md` and `crates/argot-rules-semantic/model/README.md` |
| Daily pre-acceptance auto-run | **Partially implemented** | Only automatic wiring is the **pre-write** `PreToolUse` hook (`hooks/hooks.json`, `crates/argot-cli/src/hook.rs`), Claude Code plugin only, fitted repos only, `foreign-import` only, "ask" not block. No post-generation / pre-accept auto-run. Commit-time check is manual / agent-chosen / user-wired pre-commit | The core habit the strategy is built on | **No, not yet** (do not claim Argot runs automatically at the acceptance moment) | P0 gap — see `ARGOT_PRODUCT_GAPS.md` |
| Pre-write "ask before a foreign dep" guardrail | Exists and verified | `hook.rs::assess` returns `permissionDecision: "ask"` on `foreign-import`; never blocks; no-op until fitted | The nearest real thing to acceptance-moment awareness | Yes, with qualification (Claude Code only, pre-write, ask-only, opt-in via plugin/fit) | — |
| Claude Code plugin | Exists and verified | `.claude-plugin/plugin.json` bundles six skills + `argot mcp` + the pre-write hook; install `/plugin marketplace add` then `/plugin install` | Primary distribution into an agent | Yes | — |
| MCP server (`argot mcp`) | Exists with limitations | `crates/argot-cli/src/mcp.rs`: stdio JSON-RPC, five tools; **passive** — the agent must choose to call it (`agents.md` confirms) | Proactive context, agent-driven | Yes, with qualification (passive; agent must call) | — |
| Other-agent support (Cursor, Codex, "70+") | Exists with limitations | Agent-agnostic skills + `AGENTS.md`; "70+" rides the third-party `npx skills add` installer's reach, not argot-tested integrations | Breadth of reach | Yes, with qualification (via the skills installer; not 70 argot-tested integrations) | — |
| `foreign-import` (+ `unfamiliar-callee`, `rare-tokens`, `convention`) | Exists and verified | `argot-engine/src/rules.rs`; group `voice`, always compiled | Core retention + acquisition signal | Yes | — |
| `superseded` (migrations) | Exists and verified | `rules.rs` group `voice`, default `warn` | Retention (repo moved on) | Yes | — |
| `redundant` (reinvention) | Exists and verified | group `semantic`, feature `semantic` (shipped on); needs the encoder | Retention | Yes | — |
| `misplaced` (placement) | Exists and verified | group `semantic`, feature `semantic` | Retention | Yes | — |
| `layering` (architecture) | Exists and verified | group `architecture`, feature `arch` (shipped on) | Retention + acquisition | Yes | — |
| `test-deleted` / `-disabled` / `-weakened` (integrity) | Exists and verified | group `integrity`, feature `integrity` (shipped on); `test-weakened` defaults `warn` | Acquisition (the memorable catch) | Yes | — |
| Custom / scripted rules | Exists and verified | crate `argot-rules-script`, feature `script` (shipped on); `.argot/rules/<name>/{rule.toml,check.rhai}`; `argot rules test` | Foundation (repo-owned config) + F2/F5 | Yes | — |
| Locked rules | Exists and verified | `rules.rs::resolve_locked`; tests `crates/argot-core/tests/locked_rules.rs` | Governance option (F3 seed) | Yes | — |
| `rule-tampered` (tamper evidence) | Exists and verified | `argot-engine/src/check/tamper.rs`; group `governance`, always compiled, pinned `error`, unsuppressable | Governance option (F3 seed) | Yes | — |
| `conventions` command | Exists and verified | `main.rs` `run_conventions_cmd`; naming/vocabulary/imports/type-funnels; placement in `--format json` | Rule-authoring on-ramp | Yes | — |
| "Codify this finding as a rule" | Exists with limitations | Skill `argot-suggest-rules` (evidence-assisted authoring), **not** an automatic miner or binary subcommand | Foundation (habit → encoded convention) | Yes, with qualification (assisted authoring, not auto-generation) | — |
| JSON output | Exists with limitations | `check --format json`; described as "stable schema" but no versioned schema file | Embeddability | Yes, with qualification (stable-by-intent, not a versioned schema) | Consider a versioned schema doc |
| SARIF output | Exists and verified | `check --format sarif` (SARIF 2.1.0); Action uploads to code scanning | Embeddability / CI | Yes | — |
| GitHub annotations (`--format github`) | Exists and verified | `main.rs`; Action inline PR annotations | CI | Yes | — |
| GitHub Action (non-blocking) | Exists and verified | `action.yml` composite; `fail-on-hits` default `false`; fits on base ref; job-summary card + sticky comment | Team on-ramp | Yes | — |
| Live README badge | Exists and verified | `publish-badge: true` → shields endpoint on a `badges` branch; `voice-diff --format svg/shields` | Awareness | Yes | — |
| Accumulated local history of findings | **Partially implemented** | `.argot/` persists model artifacts (`scorer-config.json`, `semantic-index.json`, `integrity.json`, `layering.json`, `health.json`, `suppressions.yaml`); `.argot/last-check.json` caches **only the most recent** run's hits (overwritten each run) | Foundation seed for F2/F3 record | **No, not yet** (do not claim a durable finding history) | P2 gap |
| Distribution (installers, npm) | Exists and verified | cargo-dist shell + powershell installers; npm `@tmonier/argot`; macOS arm64/x64, Linux x64/arm64, Windows x64 | Zero-cost install | Yes | — |
| Self-update + update check | Exists and verified | `update.rs` (`self-update` feature); `update_check.rs` opt-out GET of `version.json`, ≤1/24h | Maintenance | Yes | — |
| Telemetry / usage analytics | Exists and verified (as absence) | No analytics/telemetry code in `crates/`; only egress is the opt-out update check (+ `review` fetching a PR diff) | Privacy / neutrality (a trust asset) | Yes, with qualification (see §3) | — |
| Retention / audit-to-habit measurement | **Planned but absent** | No instrumentation exists; the North Star cannot be measured directly today | Measures the North Star | **No, not yet** (do not imply retention is observed) | See `ARGOT_STRATEGY.md` §"North Star measurability" |

---

## 2. Benchmark numbers as currently published (verified)

Source: `README.md:265-268`, backed by `landing/src/data/*.json`. Reproduce via `just bench*`
(`argot-bench`). These are the numbers a marketing agent may cite; do not invent others.

- **Foreign catch — 595/605 (98%)** when the foreign symbol is visible in the diff; **false alarms 0.29%** of 22,513 real hunks (worst corpus 1.46%). Backing: `landing/src/data/foreign.json`, `benchmarks/latest.json` (`worst_fp_existing_overfire_pct` 1.46%).
- **Architecture — 244/252 (96.8%)**, 0/140 controls, ≤2.7% over-fire (README). Note: the newer CI data file `landing/src/data/arch.json` (2026-07-20) aggregates to **264/272 (97.1%), 0/148, worst 2.7%** — the README figure is slightly stale versus the data file. Prefer the data file.
- **Reinvention — 545/584 (93.3%), median 94%** across 31 corpora; **Misplacement — 12,899/13,456 (95.9%), median 96%** across the 22 evaluable corpora (nine abstain). Raw clean-commit semantic fires are recorded separately and prior human labels are refused when their embedder differs. Backing: `landing/src/data/semantic.json` (window 150, static model).
- **Test-integrity — 144/153 (94.1%)**, 0/102 controls, 1.12% of 5,268 replayed accepted test-touching commits flagged. Backing: README + `docs/research/evidence/test-integrity-*.md`.
- **Documented blind spot:** masked foreign (a foreign symbol whose name collides with one already in use) is statistically invisible to the voice model; published, not hidden.

**Important framing caveat.** The 0.29% headline is the false-alarm rate of the base foreign
detector on real, unspliced history (temporal holdout). A separate CI file
(`benchmarks/latest.json`) reports the spliced-break-fixture **superset across all difficulty
tiers** with different framing (gated `foreign_recall` 85.6% = 620/724; `worst_fp_existing` 7.1%;
`worst_fp_new_file` 18.2%). Those higher over-fire figures are a recall-measurement artifact of the
hardest tiers, not the production false-alarm rate. What is **not** yet measured: real-world noise
across **all shipped detectors** (semantic, arch, integrity) at the acceptance moment in daily use.
See the P1 gap in `ARGOT_PRODUCT_GAPS.md`.

---

## 3. Privacy and network egress (verified)

Argot performs **no telemetry** and sends **no usage data or source code** anywhere. There are
exactly these outbound requests, all local-first-compatible:

1. **Update check** — a passive, ETag-conditional GET of `https://argot.tmonier.com/version.json`, at most once per 24h per machine, "notify, don't install." Silenced by non-tty stderr, `CI`, `--quiet`, machine output formats, `ARGOT_OFFLINE`, `ARGOT_UPDATE_CHECK=0`, or `[update] check = false`.
2. **`argot review` / `argot update`** — fetch a PR diff and release installers respectively, only when the user runs those commands.

`ARGOT_OFFLINE=1` disables all network. The precise honest claim is: **all analysis runs locally
and nothing about your code or usage leaves your machine; the only default outbound traffic is a
suppressible once-daily version check.** Avoid the unqualified "nothing ever leaves your machine" —
`argot review` and `argot update` are network commands the user invokes deliberately.

---

## 4. Detector build matrix

| Detector / mechanism | Group | Feature gate | Shipped release | Dev/CI base build |
|---|---|---|:---:|:---:|
| `foreign-import`, `unfamiliar-callee`, `rare-tokens`, `convention` | voice | none (always compiled) | Yes | Yes |
| `superseded` | voice | none | Yes | Yes |
| `redundant`, `misplaced` | semantic | `semantic` | Yes | No |
| `layering` | architecture | `arch` | Yes | No |
| `test-deleted` / `-disabled` / `-weakened` | integrity | `integrity` | Yes | No |
| custom / scripted rules | custom | `script` | Yes | No |
| `rule-tampered` | governance | none | Yes | Yes |
| locked rules (mechanism) | — | none | Yes | Yes |

All feature-gated detectors are **on in the shipped binary**; dev/CI base loops leave them off for
a lean pure-Rust build. Public claims about any of these detectors are safe because they ship.

---

## 5. Explicit unknowns (not established here)

- Real-world false-alarm/dismissal rate across all detectors at the acceptance moment in daily use.
- First-run wall-clock of `argot audit` on very large repositories (docs say "seconds to minutes").
- Actual breadth of the "70+ agents" claim beyond the third-party installer's advertised coverage.
- Whether `argot review`'s PR fetch uses the GitHub API, `gh`, or local git (not confirmed here).
- Retention and audit-to-habit conversion (no instrumentation exists; unmeasurable today).

## 6. Minor documentation inconsistencies observed (not fixed here; out of task scope)

- `skills/README.md` says "Five skills" and omits `argot-suggest-rules`; the plugin manifest and docs correctly list six.
- `README.md` architecture figure (244/252) trails the CI data file (264/272).

These are recorded for the maintainer; this task does not modify README, docs, site, or code.
