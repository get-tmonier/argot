# Argot — Strategy Card

> **Derived file.** Generated/hand-synced from `ARGOT_STRATEGY.md` (the canonical source) and
> grounded in `ARGOT_CURRENT_REALITY.md`. Do not edit independently; edit the canonical Markdown,
> then re-sync. Last synced: 2026-07-22.

Labels: **[reality]** true today · **[decision]** committed · **[requirement]** must build, not yet
real · **[hypothesis]** believed, unproven · **[option]** gated future.

---

## Behavioral invariant [hypothesis, high conviction]

Understanding code used to be a byproduct of writing it. AI weakened that. Developers increasingly
ship code they did not write and did not fully read, and better models are likely to intensify this.
Not "AI makes mistakes": awareness of what is being accepted stays valuable even when the code is
correct.

## Product job [decision]

Provide awareness at the moment a developer accepts responsibility for code they did not fully read.
A repository-grounded check, not a generic AI reviewer.

## Two engines [decision]

**Acquisition — installs you** [reality: ships]. Rare, shareable catches: weakened/deleted/disabled
tests, bypassed checks, tampered rules, spectacular finds, historical audits. `argot audit` is the
zero-setup front door.

**Retention — keeps you** [detectors: reality; accept-time habit: requirement]. Frequent awareness:
foreign deps, reinvented helpers, misplaced code, drift, crossed boundaries. Framed as awareness, not
defect detection, so it survives better models.

**The retention engine** is where Argot's long-term value is created — whether it becomes durable
infrastructure rather than a curiosity. This is a **[hypothesis]**, not a decision.

## Operating model [decision]

Audit installs you. Check-on-accept keeps you. **Caveat:** the accept-time auto-run is a
**[requirement]**; today only a pre-write `foreign-import` "ask" guardrail (Claude Code) is
automatic. Commit-time check is manual / agent-chosen / user-wired.

## North Star [decision]

Audit-to-habit conversion: run audit → install → enable recurring checks → still trust and use it
after 30 days. **Not directly measurable today** (no telemetry, by design). Use proxies (npm/Action/
plugin installs) + opt-in local metrics + qualitative research. Do not add default telemetry.

## Non-negotiables [decision]

Signal quality is existential; no default-gating detector above the noise threshold. Seconds, not
flow interruption. Brief, do not scold. Individual local check free. No default telemetry; only egress
is a suppressible update check. Config portable and user-owned. Embeddable
(CLI/JSON/SARIF/hooks). No **generative** LLM in the authoritative core (a local deterministic encoder
powers `redundant`/`misplaced` and is permitted).

## One-way doors (avoid without strong evidence)

Mandatory account or cloud for the core; paywalling the individual check or SARIF/JSON; default
telemetry; a generative LLM in the core; generic AI-review essays as findings; dashboards before
retained usage; organization-facing features before repeated demand; "AI governance" framing now; a default-gating
detector with high false positives; removing `argot audit`.

## Posture [decision]

Conviction on the foundation, options on the destination.

## Positioning [decision]

A repository-grounded check that gives awareness at the acceptance moment. Keep four layers separate:
behavioral truth, product job, memorable proof, current tool. "Voice" is brand/visual only, never the
explanation. (Current `--help`/site still say "voice" — a known positioning gap, P0-3.)

## Plausible futures [hypothesis — scenario weights, not forecasts]

- ~35–40% world-class standalone tool (not a failure).
- ~25–30% team product (teams adopt it as shared infrastructure).
- ~10–15% organization-level accountability/governance (call option, not a plan).
- ~15–20% acquired or commoditized (material downside).
- ~5–10% pre-generation convention layer for agents.

Building for a team product keeps the later options open cheaply. The tool→platform→governance
escalator is **not** inevitable [rejected assumption]. Argot is an open-source product; how value is
eventually captured is **intentionally outside this strategy's scope** (neither assumed nor rejected),
and the long-term form is deliberately left open.

## Unresolved [open questions]

Which future; whether teams adopt and rely on it together; whether verification demand erodes with trust; whether
independence is a sufficient moat; whether the acceptance moment stays put; how to measure
audit-to-habit without default telemetry; what the accept-time briefing's first lines should show.
