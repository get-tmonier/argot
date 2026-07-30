# Argot — Canonical Strategy &amp; Positioning

**Status:** Living document. The normative source of truth for product, marketing, distribution,
and brand decisions.
**Audience:** Maintainers, contributors, agents, designers, copywriters.
**Last substantive revision:** 2026-07-22 (hardening pass: reality-grounded, drift-resistant).

> Internal operating document, not marketing copy and not a homepage draft. Anyone changing
> Argot's product, site, or messaging checks their decision against this file first.

---

## 0. How to read and maintain this document

### 0.1 Normative hierarchy

1. **Normative source.** [§10 Standing Decisions](#10-standing-decisions-normative) is authoritative. A decision is binding only if it appears there, with its full record. Every other section explains, illustrates, or provides history and must not silently introduce a stronger or conflicting commitment. If prose elsewhere seems to commit Argot to something not in §10, §10 wins and the prose is a bug.
2. **Current fact.** [`ARGOT_CURRENT_REALITY.md`](./ARGOT_CURRENT_REALITY.md) is authoritative on what the product does today. When strategy prose and that file disagree on current fact, that file wins.
3. **Derived documents.** `ARGOT_STRATEGY_CARD.md` (condensed) and `ARGOT_STRATEGY.html` (rendered) are **derived from this Markdown**. Do not edit them independently. Edit this file, then regenerate or hand-sync the other two. If they disagree with this file, this file is correct.

This document, `docs/strategy/ARGOT_STRATEGY.md`, is the canonical strategy source, and its sibling
`ARGOT_CURRENT_REALITY.md` is authoritative on current product fact. The root-level `FOUNDER.md` is a
one-page operating manifesto and shortcut — not a replacement for this document or its decision
register. If `FOUNDER.md` ever conflicts with this file or `ARGOT_CURRENT_REALITY.md`, those win.

There is deliberately no build system for the derived files. Keep them in sync by hand and record
the sync in the changelog.

### 0.2 Label taxonomy (applied consistently throughout)

- **Current reality** — verified true in the product today (see `ARGOT_CURRENT_REALITY.md`).
- **Standing decision** — a committed choice we operate on. Lives in §10.
- **Product requirement** — must be built for the strategy to become true. Not real yet. Never describe it in the present tense.
- **Working hypothesis** — believed and acted on, not yet supported by sufficient evidence.
- **Future option** — a possible direction, gated. Must not drive current marketing or roadmap until its evidence gate is crossed.
- **Rejected assumption** — a belief we examined and set aside. Must not quietly reappear.
- **Open question** — genuinely unresolved.
- **Evidence required** — what observation would move a hypothesis or open a gate.

Do not remove nuance to make the narrative cleaner. Do not add nuance where §10 has already
decided. Do not describe a future experience in the present tense.

---

## Table of contents

1. [Executive summary](#1-executive-summary)
2. [Current product reality (summary)](#2-current-product-reality-summary)
3. [The behavioral invariant](#3-the-behavioral-invariant)
4. [The problem Argot solves today](#4-the-problem-argot-solves-today)
5. [The product's two engines](#5-the-products-two-engines)
6. [The daily habit and the acceptance moment](#6-the-daily-habit-and-the-acceptance-moment)
7. [Strategic positioning](#7-strategic-positioning)
8. [North Star and its measurability](#8-north-star-and-its-measurability)
9. [The trusted analytical core (definition)](#9-the-trusted-analytical-core-definition)
10. [Standing Decisions (normative)](#10-standing-decisions-normative)
11. [Product principles](#11-product-principles)
12. [Marketing principles](#12-marketing-principles)
13. [Review of absolute claims](#13-review-of-absolute-claims)
14. [Plausible future states](#14-plausible-future-states)
15. [Evidence gates](#15-evidence-gates)
16. [Product roadmap principles](#16-product-roadmap-principles)
17. [Distribution and onboarding principles](#17-distribution-and-onboarding-principles)
18. [Brand and language guidelines](#18-brand-and-language-guidelines)
19. [Do say / do not say](#19-do-say--do-not-say)
20. [Risks and failure modes](#20-risks-and-failure-modes)
21. [Open questions](#21-open-questions)
22. [Contributor decision test](#22-contributor-decision-test)
23. [Decision log](#23-decision-log)

---

## 1. Executive summary

Argot exists because of one change in how software is made. For most of software history,
understanding a piece of code was a byproduct of writing it. You could not produce code without
building a mental model of it, and reading code was how that model passed between people. AI
weakened that link. Code now enters repositories that no human fully understood, and the default
posture of an engineer is shifting from author to approver of work they did not write and did not
fully read.

**Working hypothesis (high conviction).** This shift is likely to intensify as models improve:
they produce more code, earn more trust, and get read proportionally less, so the gap between what
a developer is responsible for and what they directly understand is likely to widen. This is a
statement about behavior and is not settled fact.

Argot's durable role is to provide **awareness at the moment a developer accepts responsibility for
code they did not fully read**. Not "catch the AI's mistakes." Awareness of what is being accepted
stays valuable even when the generated code is correct.

The product has two engines. An **acquisition engine** of rare, memorable findings (an agent
weakening, disabling, or deleting a test; routing around a check) earns attention. A **retention
engine** of frequent awareness (a foreign dependency, a reinvented helper, misplaced code, a
crossed boundary) is intended to create the daily habit. Operating model: **audit installs you,
check-on-accept keeps you.** Note that the "check-on-accept" auto-run is a **product requirement**
today, not a shipped default (see §2 and §6).

Strategic posture: **conviction on the foundation, options on the destination.** We commit fully to
a clean, fast, local, repository-grounded briefing and defer commitment to any single long-term
destination until evidence justifies it.

Primary metric: **audit-to-habit conversion.** Note it is not directly measurable today without
new, opt-in instrumentation (see §8).

---

## 2. Current product reality (summary)

This section is a compact, load-bearing summary. The full verified inventory, with evidence and
public-claim guidance, is [`ARGOT_CURRENT_REALITY.md`](./ARGOT_CURRENT_REALITY.md). Do not market
anything as current unless it is confirmed there.

**Verified today (Current reality):**
- `argot audit`: zero-setup (fits a temporary model in a throwaway worktree), default 50 commits, real self-contained HTML share card and caption, exits 0. Attribution to ai-assisted / human / unknown is from **concrete commit markers** (agent emails, bot slugs, footers); "human" means "no marker found," so the AI share is a floor, not a census.
- `argot check`: real, but **requires a prior fit** (`argot init`); formats human/json/sarif/github; non-blocking by severity tier.
- Detectors shipped in the release binary: `foreign-import` (+ friends), `superseded`, `redundant`, `misplaced`, `layering`, `test-deleted/-disabled/-weakened`, custom scripted rules, locked rules, `rule-tampered`.
- CI: a non-blocking GitHub Action (score card + annotations), SARIF, JSON, GitHub annotations, live badge.
- Distribution: shell/powershell installers, npm `@tmonier/argot`; macOS arm64/x64, Linux x64/arm64, Windows x64.
- Privacy: **no telemetry**; the only default egress is a suppressible once-per-24h update check. The semantic detectors' embedding model ships inside the binary, so analysis needs no network at all.
- Agent surface: Claude Code plugin (six skills + passive MCP server + a pre-write `foreign-import` "ask" guardrail); other agents via a third-party skills installer.

**Not yet real (do not market as present):**
- **Product requirement:** Argot running automatically at the acceptance moment. Today only a *pre-write* foreign-import guardrail is automatic (Claude Code only); commit-time `check` is manual, agent-chosen, or user-wired pre-commit.
- **Product requirement:** a durable local history of findings. Today only model artifacts and a single-run `last-check.json` persist.
- **Product requirement / open question:** direct measurement of retention (no instrumentation exists).
- **Working hypothesis:** low enough noise across *all* detectors at the acceptance moment in daily use. The strong 0.29% false-alarm figure is the base foreign detector on real history; combined accept-time noise is unmeasured.

---

## 3. The behavioral invariant

**Working hypothesis (high conviction). The foundation of everything else, held as a belief, not a proof.**

For decades, writing code, understanding code, and being responsible for code were welded together.
You wrote it, so you understood it; you understood it, so you could answer for it. Reading was how
understanding moved between people, and review worked because someone who could make code look right
had usually understood it.

AI weakened the weld. Producing code is now close to free; understanding it is still bound to human
time. So the behavior is shifting: engineers increasingly ship code they have not fully read, often
rationally, because reading every line an agent writes removes the agent's benefit.

**Refined claim (was "the gap widens without bound").** The gap between responsibility and direct
human understanding is **likely to widen as agent output grows faster than human review capacity**.
Stated as a tendency, not a law.

**What this insight is not.** It is not "AI makes mistakes." Mistake detection may improve in the
models or be commoditized by the platforms. The durable need is awareness of what is being accepted,
which persists even when the code is correct.

**Why it matters.** Every scale of adoption is the same job at a different scope: help someone
stand behind code they did not fully read. That through-line is what could let Argot evolve without a
discontinuity. It is also why we do not commit early to where the evolution ends.

---

## 4. The problem Argot solves today

A developer prompts a coding agent, gets a plausible and usually fine diff, skims it because reading
all of it defeats the purpose, accepts it, and puts their name on the commit. They carry a low,
persistent unease and no efficient way to resolve it.

Type checkers and linters answer "is this valid?", a question increasingly handled inside the agent.
The unanswered question is closer to what a reviewer used to ask: "is this how we do things here, and
did anything happen in this diff I would want to know before I put my name on it?" Argot answers that
against the repository's own history and structure.

**Standing decision (D-POSITION).** Describe the job as *awareness at the moment of acceptance*,
grounded in the repository. Do not describe it as "AI code review" (implies a second model's opinion)
or as a "style/voice checker" (understates it and anchors to the shrinking axis).

---

## 5. The product's two engines

**Standing decision (D3).** Build and market two complementary engines. They are different features
and should not be collapsed into one.

**Acquisition engine (memorable, rare, shareable).** Weakened / disabled / deleted tests; bypassed or
muted checks; a tampered locked rule; a spectacular real-world catch; a historical audit. Earns
attention. The ignition, and a renewable top-of-funnel. *Current reality: the integrity detectors,
`rule-tampered`, and `argot audit` all ship.*

**Retention engine (frequent, practical, daily).** Foreign dependencies, reinvented helpers,
misplaced code, architecture drift, unusual patterns, test changes, crossed boundaries. Intended to
create the habit via a fast briefing at the acceptance moment. *Current reality: all these detectors
ship; the acceptance-moment auto-run does not (see §6).*

**Working hypothesis (not a decision): the retention engine is where Argot's long-term value is
created — it determines whether Argot becomes durable infrastructure rather than a curiosity; the
acquisition engine is the ignition.** Reasoning: habit is what turns a tool into infrastructure. This depends on user
behavior and is not yet evidenced. **Recorded minority view:** an earlier analysis argued the
memorable catch should be the whole story; rejected because a once-per-hundred-diffs tool does not
build a habit. **Standing tension:** the retention detectors partly depend on model deficiencies that
may shrink, which is why D4 points them at awareness rather than defect detection.

**Standing decision (D4).** Frame the retention engine as *awareness*, not *defect detection*. "You
are about to ship something unlike anything in your repo; did you mean to?" is true even when the
model is perfect; "the AI erred" is not.

---

## 6. The daily habit and the acceptance moment

**Standing decision (D2). Operating model: Audit installs you. Check-on-accept keeps you.**

- **The install trigger** is `argot audit`. **Current reality:** zero-setup, real, shareable, exits 0.
- **The habit** is a seconds-long briefing at the moment the developer accepts an agent's change, showing only the few parts that deserve attention.

**Product requirement (P0-1), not current reality.** Argot does **not** run automatically at the
acceptance moment today. The only automatic-on-install wiring is a *pre-write* `foreign-import` "ask"
guardrail (Claude Code plugin only, fitted repos only). Commit-time `check` is manual, agent-chosen,
or user-configured pre-commit. Do not write "Argot runs automatically before you accept." Write "the
onboarding should make Argot run at the acceptance moment" (a requirement) or describe the real
pre-write guardrail precisely.

**Standing decision (D8).** Pursue onboarding that wires the check into the acceptance moment (or the
nearest reachable agent lifecycle event) so it runs without a manual step. This is the single
highest-leverage onboarding investment. It is a direction we have committed to, and a requirement not
yet met.

**Standing decision (D14). Signal quality is existential.** A noisy briefing at accept-time is worse
than nothing; it trains people to ignore the check. No detector ships default-gating above a defined
noise threshold. **Evidence required:** measured accept-time noise across all default detectors
(P0-2, P1-2).

---

## 7. Strategic positioning

**Standing decision (D6). Posture: conviction on the foundation, options on the destination.**

**The foundation (full conviction).** A clean, low-noise daily signal; local-first operation;
repository-grounded analysis; speed in seconds; portable, repository-owned configuration;
embeddability; integration into the agent acceptance flow (a requirement, §6); a useful individual
developer experience. Every future we currently find plausible depends on this foundation.

**The destination (held open).** We do not steer toward a single endpoint. We invest only in what the
foundation needs and defer future-specific work until an evidence gate is crossed (§14, §15).

**Rejected assumption.** That Argot must become an organization-level governance platform, and that
remaining a standalone tool would be a failure. Both are false. Influence and value capture are different events
(git stayed a tool; GitHub captured the value). Neither destination is inevitable, and optimizing
prematurely for the least probable outcome can burn the capital the more probable ones depend on.

**Argot is an open-source product.** Its long-term form is deliberately left open, and how value
might eventually be sustained or captured is intentionally outside the scope of this strategy —
neither assumed nor rejected. This document answers *why the product should exist*, not *how value
should eventually be captured*; that question is deferred until there is evidence. The strategy
optimizes only for what makes an open-source tool matter: usefulness, adoption, habit, and trust.
Remaining an independent open-source tool indefinitely is a fully successful outcome.

**The four positioning layers (kept separate; §18).** Behavioral truth · product job · memorable
proof · current tool.

---

## 8. North Star and its measurability

**Standing decision (D5). Primary metric: audit-to-habit conversion.** The full path: (1) run
`argot audit`; (2) install or configure Argot; (3) enable recurring checks in the agent workflow;
(4) still trust and use it after 30 days. Stars and installs are secondary.

**The measurement conflict (stated honestly).** **Current reality:** Argot has **no telemetry** and
no way to observe steps 3 or 4. The North Star is therefore a **conceptual** target that cannot be
measured directly today without new instrumentation, and D13 forbids default telemetry. Do not imply
retention is being observed.

**Standing decision (D13-adjacent). Do not resolve this by adding default telemetry.** Distinguish:

- **The conceptual North Star** — audit-to-habit conversion (above).
- **Measurable today (proxies, no new code):** npm download counts; GitHub Action usage; plugin/marketplace installs; release-asset download counts; package-update behavior where ethically and legally available. These are weak proxies for reach, not retention.
- **Measurable only with opt-in instrumentation:** local metrics shown to the user; fully opt-in anonymous usage reporting; opt-in dismissal/false-alarm counts (P1-3). Any such instrumentation must be off by default, anonymous, and local-first.
- **Requires qualitative research:** voluntary post-install surveys; user-research cohorts; interviews on whether the accept-time briefing is kept on.

**Working hypothesis.** Dismissal rate and false-alarm rate are the leading indicators of retention;
if either climbs, retention falls before any proxy shows it. **Evidence required:** an opt-in,
privacy-compatible way to observe dismissals (P1-3).

---

## 9. The trusted analytical core (definition)

This section defines a term used as a boundary throughout. A future contributor should be able to
judge a proposed feature against it.

### 9.1 Authoritative analytical path

The logic that: detects findings; computes scores; determines severity; decides whether a rule was
violated; produces machine-readable evidence; and affects exit codes or automated enforcement.

**Current reality.** This path is statistics (frequency tables + callee clustering) plus, for
`redundant` and `misplaced`, a **local, deterministic code-embedding encoder** (a distilled
static table compiled into the binary). The encoder is a fixed model that turns code into vectors; it is not a
generative or opinion-forming model. `layering` uses a module-dependency graph; the integrity rules
use a tree-sitter test inventory. All of it runs locally and is reproducible.

**Standing decision (D12). The authoritative path must remain:** reproducible; inspectable; runnable
without any **generative** LLM; stable enough for automation; and grounded in explicit evidence a user
can check. "No LLM in the core" means **no generative or opinion-forming model** decides a finding,
score, severity, or CI outcome. It does **not** forbid the existing deterministic encoder, which is
part of the core.

### 9.2 Optional non-authoritative assistance

A future LLM **may** be used for: natural-language explanations of a finding; documentation help;
summarization; onboarding guidance; suggesting a remediation; drafting a custom rule for human review.

Such assistance must **never** silently: create an authoritative finding; alter a score or severity;
determine whether CI passes; replace the underlying evidence; or become mandatory for the core local
workflow. It must be optional, clearly marked as assistance, and removable without changing any
verdict.

**Test for a proposed feature.** If it changes what is flagged, the score, the severity, or the exit
code, it belongs to the authoritative path and may not be an LLM. If it only explains, summarizes, or
drafts-for-review, it may be an LLM, provided it is optional and non-authoritative.

---

## 10. Standing Decisions (normative)

This register is the authoritative source. Each decision: ID, statement, status, rationale, the
evidence that would reverse it, and the date last substantively revised. Prose elsewhere may explain
these but may not exceed them.

| ID | Statement | Status | Rationale | Reversal evidence | Last revised |
|---|---|---|---|---|---|
| **D1** | The behavioral invariant (§3) is the foundational belief and root of positioning. | Active (foundational working hypothesis) | Strongest insight found; reframes the product away from shrinking axes. | Evidence that developers do not, in aggregate, ship increasing volumes of unread AI code. | 2026-07-22 |
| **D2** | Operating model: audit installs, check-on-accept retains. | Active; the accept-time auto-run is a Product requirement, not yet real. | Habit must ride the existing accept reflex. | Audit-to-habit conversion stays low after the accept-time integration ships. | 2026-07-22 |
| **D3** | Build and market two engines (acquisition + retention); do not collapse them. | Active | They do different jobs (attention vs habit). | Evidence that one engine alone sustains both acquisition and retention. | 2026-07-22 |
| **D4** | Frame the retention engine as awareness, not defect detection. | Active | Defect framing ties value to model deficiencies that may shrink. | Evidence that users only value catches, not awareness of correct-but-unfamiliar code. | 2026-07-22 |
| **D5** | North Star is audit-to-habit conversion. | Active; not directly measurable today (§8). | Retention predicts durability and real use; stars do not. | A better-correlated, measurable retention metric is found. | 2026-07-22 |
| **D6** | Posture: conviction on the foundation, options on the destination. | Active | Several futures are plausible; premature commitment burns capital. | One future's evidence gate (§15) is decisively crossed. | 2026-07-22 |
| **D7** | The fully local individual core check remains free and requires no account or payment. | Active (standing commitment) | Free individual use is the shared prerequisite of every future. | A deliberate, documented reversal; none currently intended. | 2026-07-22 |
| **D8** | Pursue onboarding that makes the check run at the acceptance moment without a manual step. | Active; requirement not yet met (P0-1). | The habit must not depend on memory. | The accept-time integration proves not to move retention. | 2026-07-22 |
| **D9** | Build no future-specific feature (dashboards, governance, organization-facing work) before its evidence gate (§15) is crossed and recorded. | Active | Optionality is cheap only if branch-work is deferred. | A gate is recorded as crossed. | 2026-07-22 |
| **D10** | "Voice" is demoted from positioning to a secondary brand and visual metaphor; it never carries the explanatory load. | Active | Describes the shrinking style axis; undersells trust and awareness. | Research shows "voice" is the phrase users actually adopt (§18). | 2026-07-22 |
| **D11** | Keep the four positioning layers separate; do not compress them into one slogan. | Active | Collapsing them produces vague copy. | — | 2026-07-22 |
| **D12** | No generative or opinion-forming model in the authoritative analytical core (§9). The existing local deterministic encoder is part of the core and is permitted. | Active | The core's value is reproducible evidence, not another model's opinion. | — (a deliberate architecture change, explicitly recorded). | 2026-07-22 |
| **D13** | Local-first with no default telemetry. The only default egress is a suppressible once-per-24h update check. Any retention measurement must be opt-in, anonymous, local-first. | Active | Neutrality and privacy are non-retrofittable trust assets. | A deliberate, documented reversal; none intended. | 2026-07-22 |
| **D14** | Signal quality is existential; no detector ships default-gating above a defined noise threshold. | Active | A noisy accept-time check is worse than nothing. | — (threshold may be tuned; the principle holds). | 2026-07-22 |

---

## 11. Product principles

Operational consequences of §10. Where a principle restates a decision, the decision governs.

1. **Signal quality is existential (D14).** Protect it above feature count.
2. **Noise destroys the product.** A dismissed finding is a withdrawal from trust.
3. **Seconds, not flow interruption.** Speed ranks with correctness.
4. **Attach to an existing behavior, not a new ritual (D8).**
5. **Brief, do not scold.** Situational awareness, not lint shame.
6. **The individual local check stays free (D7).**
7. **Cloud is never mandatory for the core check (D13).**
8. **Local-first and neutrality are non-retrofittable trust assets (D13).**
9. **Configuration is portable and user-owned.** *Current reality: commit the reviewed `argot.toml` and `.argot/` fit snapshot; CI reads the approved base snapshot and never fits or caches it. Hand-written `.argot/rules/` custom rules remain committed source.*
10. **Stay embeddable.** Stable CLI, JSON, SARIF, hooks, agent integrations. *Note: JSON is stable-by-intent; a versioned schema is a P2 gap.*
11. **No generative LLM in the authoritative core (D12).**
12. **Do not reposition as "AI governance" prematurely (D6, D9).**
13. **Build organization-facing features only after repeated, concrete inbound demand (D9, §15).**

---

## 12. Marketing principles

1. **Lead with the behavior, not a metaphor.**
2. **One message, everywhere.** The failure mode being corrected is many competing taglines.
3. **Acquisition through stories, retention through habit; never let the memorable engine become the whole pitch.**
4. **Proof over adjectives.** Real catches, honest method, and the published blind spot persuade.
5. **State caveats once, confidently.**
6. **Do not tie the brand to model deficiencies (D4).**
7. **Respect attention; one idea per screen.**
8. **The prose must not read like the machine slop the product catches.** Short sentences, few em dashes.
9. **Never market a Product requirement or Future option in the present tense (§0.2).** Consult `ARGOT_CURRENT_REALITY.md` before making a capability claim.

---

## 13. Review of absolute claims

Each previously-absolute claim, reviewed. Keep an absolute only when it is a deliberate principle,
within Argot's control, its reversal cost is understood, and the wording does not block reasonable
future change. Otherwise rewrite as a hypothesis.

| Original claim | Verdict | Kept / rewritten as |
|---|---|---|
| "The gap widens without bound." | Rewrite (depends on behavior) | Working hypothesis: "likely to widen as agent output grows faster than human review capacity" (§3). |
| "The individual daily check is free forever." | Keep, made precise | Standing commitment D7: the fully local individual core check remains free and requires no account or payment. |
| "Never add an LLM." | Keep, scoped precisely | D12: no **generative** model in the authoritative core; the local deterministic encoder is permitted (§9). |
| "Agents will always cheat." | Rewrite | Working hypothesis: incentive-gaming has a longer half-life than capability-drift; a constrained/aligned agent may game less. Tracked in §20. |
| "Every future requires the foundation." | Keep, softened | "Every future we currently find plausible depends on the foundation" (§7). |
| Future-state probabilities. | Reframed | Scenario-weighting tools, not forecasts (§14). |
| "The retention engine is the company." | Downgraded + reworded | Working hypothesis; reworded to drop the company framing — it determines whether Argot becomes durable infrastructure (§5). |
| "Audit installs you." | Keep | Current reality: audit is zero-setup and shareable (§6, reality doc). |
| "Check-on-accept keeps you." | Split | Operating-model target (D2) + Product requirement for the accept-time auto-run (§6, P0-1). |
| "Acceptance is the permanent attach point." | Rewrite | Open question: the attach point may move as agent workflows evolve (§21). |

---

## 14. Plausible future states

**Working hypothesis / scenario-weighting tools, not empirical forecasts.** The percentages force
relative weighting and honest comparison; they are not measured probabilities. Each carries a
qualitative tag, the reasoning, current evidence, and what would update it.

**Not a roadmap.** These are possible *product* futures and honest risks (F4 is a risk, not a goal),
not a plan we steer toward. How value might eventually be captured is outside the scope of this
strategy; the project steers only by usefulness and adoption.

| # | Future | Weight (scenario tool) | Qualitative | Reasoning &amp; what would update it |
|---|---|:---:|---|---|
| F1 | World-class standalone developer tool | 35–40% | Most plausible | Base rate for beloved single-purpose tools, and Argot's magic is a tool interaction. **Updates up** if love/retention are high but team WTP is weak; **down** if teams pull toward centralization. |
| F2 | Team engineering-quality product | 25–30% | Plausible | Teams adopt Argot as shared infrastructure through committed config. **Up** if teams commit config unprompted and rely on it together; **down** if team-level adoption never appears. |
| F3 | Organization-level accountability / governance layer | 10–15% | Lower probability, high upside | Externally dependent (regulation, liability), hardest to build. A call option, not a plan. **Up** on repeated procurement/insurance/regulatory demand for AI-code controls; **down** if trust in models makes verification demand collapse. |
| F4 | Acquisition or commoditization | 15–20% | Material downside | Incumbent bundling or model-native alternatives thin the findings. **Up** if native "fits your repo" checks ship and adoption stalls; acqui-hire is a non-catastrophic floor. |
| F5 | Pre-generation convention/context layer for agents | 5–10% | Emerging option | Argot flips from checking output to feeding conventions before generation. **Up** if agents/IDEs pull the convention model from the generation side. |

**Notes.** F1 is a good outcome, not a failure. F2 and F3 share their early roadmap, so building for
F2 keeps F3 open at little extra cost (config-in-repo, a local record, neutrality). **Rejected
assumption:** that the tool→platform→governance escalator is inevitable.

---

## 15. Evidence gates

Until a gate is crossed and recorded in the decision log, we default to strengthening the foundation.

**Team platform (F2):** teams commit config unprompted; multiple contributors inherit one setup;
users ask for shared policy or cross-repo visibility; teams ask for centralized records/reports;
sustained shared reliance on a collaboration layer, distinct from the individual check.

**Accountability / governance (F3):** customers explicitly ask for tamper evidence, attestations, or
provenance; procurement/insurance/regulation asks for proof of AI-code controls; a need to
demonstrate agents could not bypass rules; the same demand recurs across unrelated customers.

**Remain a standalone tool (F1):** high individual love and retention; weak team coordination need;
users value the accept-time briefing over centralization; platform additions reduce simplicity
without lifting retention.

**Reassess the core thesis (toward F4 or a pivot):** audit-to-habit stays low after the accept-time
integration ships; users disable the check for noise; model-native alternatives remove most useful
findings; the tool rarely finds anything actionable; the acceptance moment moves or disappears.

**Pre-generation flip (F5):** agents/IDEs ask to consume the convention model to write code up front;
pull comes from the generation side.

---

## 16. Product roadmap principles

**Standing decision (D9).** Nail the shared foundation; keep branch points cheap and open; exercise a
branch only when its gate is crossed. No future-specific investment before its gate.

**Transmission mechanism (current reality, partially).** The config file committed to the repo carries
Argot from one developer to a team, as ESLint/Prettier configs do. *Note:* one-click "codify a
repeated finding as a rule" is a P2 gap (`argot-suggest-rules` is assisted authoring today, not
auto-generation); a durable local finding record is a P2 gap.

Illustrative sequencing, held loosely: win the accept-time briefing (P0-1, P0-2) → let committed
config spread to teams → become part of the definition of done in CI → only if gates cross, shared
records or attestation.

The prioritized gap list is [`ARGOT_PRODUCT_GAPS.md`](./ARGOT_PRODUCT_GAPS.md).

---

## 17. Distribution and onboarding principles

1. **Front door: `argot audit`**, zero-setup, on the user's own repo. *Current reality.*
2. **Activation goal is the habit**, not the install (§8 measurability caveat applies).
3. **Product requirement:** default install should make the check fire at the acceptance moment (P0-1). Today the automatic piece is a pre-write foreign-import guardrail (Claude Code); do not overstate it.
4. **Developer-first, bottom-up.** The individual developer is the customer the whole way up. No strategy that abandons them is correct.
5. **Zero cost to try.** Free, local, non-blocking.
6. **Distribution through embeddability.** Stable interfaces are a growth lever.
7. **The committed config is the team on-ramp.** Keep it minimal and safe to commit.

---

## 18. Brand and language guidelines

### 18.1 Keep four layers separate (D11)

| Layer | Statement | Where it lives |
|---|---|---|
| Behavioral truth | Developers are responsible for code they increasingly do not read. | "Why we exist" narrative, essays, talks. |
| Product job | Awareness at the moment you accept that code. | Headlines, primary value proposition. |
| Memorable proof | An agent may weaken a test or route around a check. | Launch stories, "Caught in the Wild", social. |
| Current tool | A repository-grounded check, not a generic AI reviewer. | Docs, honest description, comparison. |
| Optional long-term | A record and control layer, only if users pull us there. | Not in current marketing. |

### 18.2 The "voice" metaphor (D10)

**Decision: demote, do not delete.** Reasoning, not deference: as positioning, "voice" describes
style and fit (the shrinking axis) and is aesthetic where the stakes are trust and awareness; it was
also one of several competing taglines. But the name *argot* means a group's private vocabulary, so
vocabulary/voice imagery has real brand coherence and is distinctive. Resolution: "voice" may appear
in visual identity, the name's story, and the emotional register, but never as the headline or the
sentence explaining what the product does. **Note (current reality):** the shipped `--help` tagline
and site still lead with "voice"; aligning them is out of scope for this document but recorded as a
P0-3 positioning gap. **Evidence required** to reverse: research showing "voice" is the phrase users
actually repeat.

### 18.3 Tone

Direct, concrete, honest, professional. No inflated claims, no consultant register, few em dashes.
State the blind spot; state caveats once; let proof carry the rest.

---

## 19. Do say / do not say

| Do say | Do not say | Why |
|---|---|---|
| "Awareness at the moment you accept AI code." | "AI code review." | "Review" implies a second model's opinion; the core is evidence. |
| "It shows what the agent introduced that deserves a look." | "It catches the AI's mistakes." | Awareness survives correct code (D4). |
| "Grounded in your repository's own history." | "Your codebase has a voice." | The grounding explains; the metaphor is brand-only (D10). |
| "A repository-grounded check." | "An AI governance platform." | Premature governance framing (D6). |
| "Runs locally; reproducible; no generative LLM in the core." | "Deterministic verification is our moat." / "No models at all." | Determinism is a real advantage but erodable as a moat; and a local encoder IS used (§9). |
| "The onboarding aims to run it at the acceptance moment." | "Argot runs automatically before you accept." | Accept-time auto-run is a Product requirement, not shipped (§6). |
| "All analysis is local; the only default egress is a suppressible update check." | "Nothing ever leaves your machine." | Precise and true (§8, reality doc §3). |
| "The individual local check is free." | (anything implying it may be paywalled) | D7. |
| "Attributed from commit markers; a floor, not a census." | "It knows which code the AI wrote." | Attribution is marker-based and a floor (reality doc §1). |
| "Here is what we cannot catch, and why." | (hiding the blind spot) | Honesty is a trust asset. |

---

## 20. Risks and failure modes

- **Noise kills the habit (largest risk).** A check turned off in week two ends every future. Mitigation: D14; watch dismissal/false-alarm as leading indicators (needs P1-3).
- **Accept-time integration never becomes automatic.** The habit stays dependent on memory. Mitigation: P0-1 is the top gap.
- **Retention findings erode as models improve.** Mitigation: D4 (awareness framing). Open whether it fully holds.
- **Incumbent bundling.** Partial defense: independence and the accept-time habit; concede bundled-good-enough wins low/mid stakes.
- **Verification demand erodes with trust (the compiler analogy).** Unresolved (§21).
- **Premature platform/governance investment.** Mitigation: §15 gates.
- **Positioning drift back to "voice" or to over-claiming unshipped features.** Mitigation: §0, §19, and `ARGOT_CURRENT_REALITY.md`.
- **Over-reliance on stars.** Mitigation: D5 (retention, not stars) — while acknowledging the measurement gap (§8).

---

## 21. Open questions

1. Which future (F1–F5) does Argot end up in?
2. Is there enough team-level demand and shared reliance to make a team product (F2) worthwhile?
3. Does verification demand persist, or erode toward a regulated niche as trust in models rises?
4. Is independence a sufficient moat for the project, or only a category moat (needing trust-brand + ubiquity)?
5. Does the acceptance moment stay where it is, or move as agent workflows evolve?
6. What exactly should the accept-time briefing show in its first lines so busy engineers keep it on? (Evidence required; P1-2.)
7. Is the code-level slice where accountability value concentrates, or does it sit in agent actions/permissions?
8. How do we measure audit-to-habit conversion without default telemetry (§8)?

---

## 22. Contributor decision test

A future contributor should be able to decide, from this document alone, whether a change is allowed.
Worked answers for ten representative proposals. Cite the governing decision.

| # | Proposal | Verdict | Governing basis |
|---|---|---|---|
| 1 | Add an optional LLM explanation command | **Allowed** | D12 / §9.2: permitted if non-authoritative (does not create findings, alter score/severity/exit code, replace evidence, or become mandatory). |
| 2 | Require an account or cloud for the core check | **Not allowed** | D7, D13; one-way door. The core check must run fully local and free. |
| 3 | Add a shared team dashboard | **Gated** | D9 + §15 F2. Only after the team gate is crossed and recorded; the individual local check stays free and unrestricted (D7). |
| 4 | Add telemetry to compute 30-day retention | **Not allowed by default** | D13. Only opt-in, anonymous, local-first instrumentation is permissible (§8). |
| 5 | Change the hero back to "Your codebase has a voice" | **Not allowed** | D10. "Voice" is brand-only, never the explanatory headline. |
| 6 | Add a new detector with a 4% false-positive rate, default-gating | **Not allowed** | D14. May ship only as `warn`/`off`/opt-in, never default-gating above the noise threshold. |
| 7 | Introduce a plugin that runs before the agent writes | **Allowed / exists** | §6; consistent with the opt-in, non-blocking pre-write guardrail already shipped. |
| 8 | Lock SARIF output behind an account | **Not allowed** | Principle 10 (embeddability); breaks CI and distribution. |
| 9 | Build an attestation / provenance export | **Gated** | D9 + §15 F3. Not before repeated inbound organizational demand, recorded. |
| 10 | Remove `argot audit` | **Not allowed** | D2/D5; audit is the verified acquisition front door and North Star step 1. |

If a real proposal is not clearly answerable from §10 and §9, that is a gap in this document; improve
the document rather than guessing.

---

## 23. Decision log

Newest first. Each entry: what changed, why, and what would reverse it.

**2026-07-22 — Open-source framing pass.** Corrected language that implicitly assumed Argot is a
company or startup. Argot is an open-source product whose long-term form is deliberately undecided.
Reframed "retention engine is the company," "durable business," "highest-EV business / SaaS arc," and
"company moat" into project/product/adoption language, and moved value-capture out of the narrative:
the strategy stays **agnostic** about how value is eventually captured (neither assumed nor rejected)
and answers only why the product should exist. Internal evidence gates (§15) and the free-check
commitment (D7) are unchanged in substance. No strategic conclusion, product decision, roadmap, or
gate changed. Full before/after in `ARGOT_STRATEGY_CHANGELOG.md`.

**2026-07-22 — Hardening pass (reality-grounded).** Verified every product claim against the
repository (see `ARGOT_CURRENT_REALITY.md`). Substantive changes: reframed accept-time auto-run as a
Product requirement (was implied present); scoped D12 to "no generative model in the core" (a local
deterministic encoder is used and permitted); qualified local-first with the update check + model
download; downgraded "retention engine is the company" and future probabilities to hypotheses/
scenario tools; made D7 (free) and D13 (no default telemetry) precise; added §8 measurability
conflict, §9 trusted-core definition, §10 normative decision register, §22 contributor test. Full
before/after in `ARGOT_STRATEGY_CHANGELOG.md`.

**2026-07-22 — D10.** Demote "voice" to brand/visual layer. Reversible on evidence users adopt the word.

**2026-07-22 — D6.** Adopt "conviction on foundation, options on destination"; reject governance-inevitability.

**2026-07-22 — D4.** Point retention engine at awareness, not defect detection.

**2026-07-22 — D5.** North Star is audit-to-habit conversion (measurability caveat, §8).

**2026-07-22 — D2 / D8.** Operating model "audit installs, check-on-accept keeps"; pursue accept-time wiring.

**2026-07-22 — D3.** Two engines; "retention is the company" recorded as hypothesis, not decision.

**2026-07-22 — D12 / D13 / D14.** Trusted-core boundary, no-default-telemetry, signal-quality-existential.

**2026-07-22 — D1.** Adopt the behavioral invariant as the foundational working hypothesis.
