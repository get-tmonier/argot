# Argot — Product Gaps

**Purpose.** The gap between the canonical strategy (`ARGOT_STRATEGY.md`) and the verified product
(`ARGOT_CURRENT_REALITY.md`). Not a speculative roadmap. Each gap is what must (or must not) change
for the strategy to be safe to execute.

**Priorities:** P0 blocks the repositioning · P1 blocks retention · P2 strengthens the foundation ·
P3 evidence-gated (do not build until a gate is crossed) · Rejected (do not build now).

**Sizing (relative only):** Small · Medium · Large · Unknown. No time estimates are invented; the
repository does not provide enough evidence for them.

Each gap uses this shape: **Current reality → Desired outcome → Strategic reason → North Star step
affected → Evidence → Scope → Dependencies → Preconditions → Success measure.**

---

## P0 — Blocks the repositioning

### P0-1. Acceptance-moment integration is not automatic
- **Current reality.** The only automatic-on-install wiring is a *pre-write* `PreToolUse` hook (Claude Code plugin only, fitted repos only, `foreign-import` only, "ask" not block). Commit-time / acceptance checking requires a manual `argot check`, the agent choosing to run the `argot-check` skill, or the user wiring pre-commit themselves. (`hooks/hooks.json`, `hook.rs`, `.pre-commit-hooks.yaml`, `agents.md`.)
- **Desired outcome.** After an agent finishes a change, Argot's briefing appears at the moment the developer decides to accept it, with no manual step and no bespoke setup, across the supported agents.
- **Strategic reason.** "Check-on-accept keeps you" is the retention engine. If it is not wired to the acceptance moment, the habit depends on memory, and the North Star cannot form.
- **North Star step affected.** Step 3 (recurring checks in the agent workflow) and step 4 (30-day retention).
- **Evidence.** The strategy's own operating model; the reality that no post-generation/pre-accept auto-run exists today.
- **Scope.** Large (varies by agent surface; Claude Code post-write/stop hooks vs other agents' capabilities differ).
- **Dependencies.** A fitted repo; fast base check; clean output (P1-1, P1-2).
- **Preconditions.** Confirm which agent lifecycle events (post-write, stop, pre-commit) are reachable per agent without a bespoke user setup.
- **Success measure.** Share of plugin/skill installs where a check actually fires at accept-time without the user configuring anything; then its 30-day survival.

### P0-2. Signal quality is unproven at the acceptance moment across all shipped detectors
- **Current reality.** The strong, published numbers are the base foreign detector (0.29% false alarms on real history). Real-world noise across semantic, arch, and integrity detectors, at the acceptance moment, in daily use, is not measured. The harder-tier superset shows materially higher over-fire (7.1%/18.2%) in a different framing. (`ARGOT_CURRENT_REALITY.md` §2.)
- **Desired outcome.** A defensible, measured statement that the acceptance-moment briefing, with all default detectors on, is quiet enough to keep on.
- **Strategic reason.** Signal quality is existential (Standing Decision). A noisy briefing at accept-time is worse than nothing and kills the habit.
- **North Star step affected.** Step 4 (retention / trust).
- **Evidence.** Existing per-detector benchmarks are strong but partial; no combined acceptance-moment measurement exists.
- **Scope.** Medium (measurement first, then tuning).
- **Dependencies.** A way to observe dismissals (P1-3) without violating local-first.
- **Preconditions.** Decide the default detector set and default confidence tier shown at accept-time.
- **Success measure.** Measured dismissal/false-alarm rate at accept-time under a defined threshold; the briefing's first lines judged actionable by real engineers.

### P0-3. Positioning does not yet match the product's honest description
- **Current reality.** The binary and site still lead with "voice linter / your codebase has a voice." The strategy demotes "voice" to a brand layer and leads with awareness-at-acceptance. (Out-of-scope to change here, but it is a gap.)
- **Desired outcome.** Public surfaces (README, site, `--help` tagline) describe the product as awareness at the acceptance moment, grounded in the repo, with "voice" as brand only.
- **Strategic reason.** D10; the four-layer messaging model.
- **North Star step affected.** Steps 1–2 (discovery, install).
- **Evidence.** `argot --help` tagline "Voice linter…"; README H1.
- **Scope.** Medium (copy, not code) — and explicitly deferred; this task does not modify public surfaces.
- **Dependencies.** None technical.
- **Preconditions.** The strategy is settled (it is).
- **Success measure.** Discovery surfaces state the awareness job in one consistent sentence.

---

## P1 — Blocks retention

### P1-1. Setup friction precedes the habit
- **Current reality.** `argot check` requires a prior `argot init`/fit (errors "run `argot init` first"). The audit front door is zero-setup, but the daily check is not.
- **Desired outcome.** The path from the audit "aha" to a fitted, checking repo is one obvious, near-automatic step (the setup skill exists; make it the default, low-friction path).
- **Strategic reason.** The North Star is a funnel; fit is the step where users drop.
- **North Star step affected.** Step 2 → 3.
- **Evidence.** `load.rs` cold-run error; `getting-started.md`.
- **Scope.** Medium.
- **Dependencies.** —
- **Preconditions.** —
- **Success measure.** Audit → fitted-repo conversion rate.

### P1-2. Output must brief, not scold, in seconds
- **Current reality.** Output is detailed and evidence-rich; whether the first three lines read as an actionable briefing at accept-time is untested (an explicit strategy Evidence-required item).
- **Desired outcome.** A default accept-time view that a busy engineer reads in seconds and acts on.
- **Strategic reason.** Brief-don't-scold; seconds-not-flow-interruption (Standing Decisions).
- **North Star step affected.** Step 4.
- **Scope.** Medium.
- **Success measure.** Real-engineer test: kept-on rate after first week.

### P1-3. Dismissal / suppression signal is not observable (privacy-compatible)
- **Current reality.** `.argot/last-check.json` caches only the most recent run; suppressions exist (`argot mute`, inline, `[exclude]`) but there is no privacy-compatible way to learn aggregate dismissal rates.
- **Desired outcome.** An opt-in, local-first way to see (and, if the user opts in, aggregate) how often findings are acted on vs dismissed.
- **Strategic reason.** Dismissal rate is the leading indicator of retention (Working Hypothesis).
- **North Star step affected.** Step 4 (measurement).
- **Scope.** Medium. **Dependency/precondition:** must not violate no-default-telemetry (see strategy §"North Star measurability").
- **Success measure.** A defensible, opt-in dismissal metric exists without default telemetry.

---

## P2 — Strengthens the foundation (shared by several futures)

### P2-1. Durable local finding history
- **Current reality.** Only model artifacts and a single-run `last-check.json` persist; no append-only finding log. Longitudinal analysis is done by replaying history (`argot audit`).
- **Desired outcome.** An optional, local, portable record of findings over time in `.argot/`, useful to the developer today (see your repo's drift), and the substrate for F2/F3 later.
- **Strategic reason.** A cheap option that keeps F2 (team records) and F3 (governance record) open without building either.
- **North Star step affected.** Indirect (enables future measurement and the record product).
- **Scope.** Medium. **Precondition:** must stay local, portable, user-owned, gitignore-safe.
- **Success measure.** A stable local record exists that a future feature can read without a new data model.

### P2-2. Stable, versioned machine-readable schema
- **Current reality.** JSON is "stable by intent"; no versioned schema file. SARIF is standard.
- **Desired outcome.** A documented, versioned JSON schema so integrators can depend on it.
- **Strategic reason.** Embeddability is a foundation principle and a distribution lever.
- **Scope.** Small.
- **Success measure.** A published schema + version field; no silent breaking changes.

### P2-3. Rule codification from repeated findings
- **Current reality.** `argot-suggest-rules` is assisted authoring off `argot conventions`; there is no one-click "codify this repeated finding as a rule."
- **Desired outcome.** From a recurring finding, offer to write the contrapositive rule (fixture-gated), turning habit into committed convention.
- **Strategic reason.** The transmission mechanism from individual habit to team config (F2), at low cost.
- **Scope.** Medium.
- **Success measure.** Rules created from observed findings; committed configs increase.

### P2-4. Broaden genuine acceptance-moment integration beyond Claude Code
- **Current reality.** The pre-write guardrail is Claude Code only; other agents use passive MCP.
- **Desired outcome.** A real accept-time (or pre-write) integration for the other major agents where their lifecycle allows.
- **Strategic reason.** The habit should not depend on one agent.
- **Scope.** Large (per-agent). **Precondition:** confirm each agent's reachable lifecycle events.
- **Success measure.** Accept-time checks firing on more than one agent without bespoke setup.

---

## P3 — Evidence-gated options (do not build until the gate is crossed)

Gates are defined in `ARGOT_STRATEGY.md` §13. Until crossed, these are off the roadmap.

- **P3-1. Team dashboards / cross-repository visibility** — gate: F2 (teams commit config unprompted; ask for shared visibility; willingness to pay for collaboration).
- **P3-2. Hosted records / shared policy service** — gate: F2, and only as opt-in, never mandatory for the local check.
- **P3-3. Organization management, SSO, enterprise controls** — gate: F3 (repeated inbound enterprise pull).
- **P3-4. Compliance / attestation / provenance export** — gate: F3 (procurement/insurance/regulation asks; same demand across unrelated customers).
- **P3-5. Cross-repo reporting** — gate: F2/F3.

For each, the precondition before any work: the corresponding evidence gate is documented as
crossed in the decision log, with the concrete signals that crossed it.

---

## Rejected / do not build now

These conflict with a Standing Decision or a one-way door in `ARGOT_STRATEGY.md`.

- **Required account or cloud service for the core check.** One-way door. The local check must run fully offline and free.
- **Default telemetry (including to measure retention).** Violates no-default-telemetry. Only opt-in, anonymous, local-first instrumentation is permissible.
- **Paywalling the individual local check, or SARIF/JSON output.** Breaks free-individual-use and embeddability.
- **A generative LLM in the trusted analytical core.** Violates the trusted-core boundary (`ARGOT_STRATEGY.md` §"Trusted analytical core"). LLMs may only assist non-authoritative surfaces.
- **Repositioning as "AI governance" / compliance now.** Premature; bets the brand on the least probable future.
- **A default-gating detector with a high false-positive rate** (e.g. ~4%). Violates signal-quality. Such a detector may only ship as `warn`/`off`/opt-in, never default-gating.
- **Enterprise sales motion.** Not before repeated, concrete inbound pull (F3 gate).

Any proposal matching this list should be refused by referencing the specific Standing Decision or
one-way door, not re-litigated.
