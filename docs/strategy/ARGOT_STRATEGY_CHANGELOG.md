# Argot Strategy — Hardening Changelog

Record of substantive changes made in the 2026-07-22 hardening pass, which verified the strategy
documents against the actual repository. Each entry: previous wording/assumption → revised wording →
reason → supporting repository evidence → impact on future marketing or product work.

The verification base is `ARGOT_CURRENT_REALITY.md`. Files changed: `ARGOT_STRATEGY.md` (canonical),
`ARGOT_STRATEGY.html` (rendering), `ARGOT_STRATEGY_CARD.md` (card). Files created:
`ARGOT_CURRENT_REALITY.md`, `ARGOT_PRODUCT_GAPS.md`, this file. No public website, README, docs,
branding, or application code was modified.

---

## Substantive changes

### C1 — Acceptance-moment auto-run: present tense → Product requirement
- **Previous.** "Default onboarding wires Argot into the agent loop so the check fires at the acceptance moment." "Check-on-accept keeps you," implying a shipped default.
- **Revised.** Marked as **Product requirement (P0-1)**, not current reality. Real today: only a *pre-write* `foreign-import` "ask" guardrail (Claude Code plugin, fitted repos). Commit-time check is manual, agent-chosen, or user-wired pre-commit.
- **Reason.** The strategy's central retention mechanism was described as if it existed.
- **Evidence.** `hooks/hooks.json` (single `PreToolUse` hook), `crates/argot-cli/src/hook.rs` (`ask` only, foreign-import only), `.pre-commit-hooks.yaml` (user must wire), `landing/src/content/docs/agents.md` ("MCP is passive; the agent has to choose to call"). Verified by subagent inspection and `argot --help`.
- **Impact.** Do not claim Argot runs automatically at acceptance. P0-1 is now the top product gap gating the repositioning.

### C2 — "Never add an LLM" → "No generative model in the trusted core"
- **Previous.** "Never add an LLM to the trusted analytical core." Card: "No LLM in the trusted core."
- **Revised.** D12 scoped to **no generative or opinion-forming model** in the authoritative path; the existing **local deterministic code-embedding encoder** (jina-code) is part of the core and permitted. New §9 defines the boundary and a per-feature test.
- **Reason.** The blanket claim was factually wrong: the core already uses a local encoder for `redundant`/`misplaced`.
- **Evidence.** `crates/argot-rules-semantic/src/embedder.rs` (jina GGUF, statically linked llama.cpp), `detector.rs` (self-gates, degrades offline). Base voice/arch/integrity are model-free.
- **Impact.** Contributors can now evaluate LLM-adjacent features against a precise boundary (explain/summarize allowed if non-authoritative; findings/scores/severity/exit code must not be model-decided).

### C3 — "Nothing leaves your machine" → qualified local-first (D13)
- **Previous.** Implied absolute locality ("nothing leaves your machine").
- **Revised.** D13: no telemetry; the **only default egress** is a suppressible once-per-24h update check and a one-time ~100 MB model download. `ARGOT_OFFLINE=1` disables all network.
- **Reason.** There are two default outbound requests; the absolute claim was imprecise.
- **Evidence.** `crates/argot-cli/src/update_check.rs` (GET `argot.tmonier.com/version.json`, ETag, ≤1/24h, opt-outs), `crates/argot-rules-semantic/src/embedder.rs` (model download). No telemetry/analytics code found in `crates/` (grep verified).
- **Impact.** Marketing may say "no telemetry; the only default egress is a suppressible update check and a one-time model download," not "nothing ever leaves your machine."

### C4 — North Star measurability made explicit (new §8)
- **Previous.** Audit-to-habit conversion presented as the metric, with supporting metrics listed as if observable.
- **Revised.** Added the measurement conflict: **no telemetry exists, so retention is not directly measurable today**, and D13 forbids default telemetry. Split into conceptual North Star / today's proxies / opt-in instrumentation / qualitative research.
- **Reason.** The prior version implied retention could be observed; it cannot without new, opt-in instrumentation.
- **Evidence.** Absence of any telemetry or retention-measurement code (verified); `.argot/last-check.json` caches only the most recent run (`crates/argot-engine/src/suppress/last_check.rs`).
- **Impact.** Prevents a future team from assuming retention dashboards exist or from adding telemetry to build them. Creates gaps P1-3 (opt-in dismissal signal).

### C5 — Future probabilities → scenario-weighting tools
- **Previous.** Numeric probability ranges presented plainly.
- **Revised.** Kept the ranges (Option A) but labeled them **scenario-weighting tools, not empirical forecasts**, and added, per future, a qualitative tag, the reasoning, and what would update it.
- **Reason.** Numeric ranges imply false precision if unqualified.
- **Evidence.** N/A (strategic estimate).
- **Impact.** Future readers treat the numbers as relative weights, not data; each has an explicit updating condition.

### C6 — "Retention engine is the company" → Working hypothesis
- **Previous.** Stated as a decision.
- **Revised.** Downgraded to a **Working hypothesis** (depends on user behavior; unproven). The two-engine split remains a decision (D3).
- **Reason.** It is a belief about market dynamics, not a controllable choice.
- **Impact.** Keeps the door open to the acquisition engine mattering more than assumed; prevents over-investment justified by an unproven premise.

### C7 — "The gap widens without bound" → bounded hypothesis
- **Previous.** Absolute.
- **Revised.** "Likely to widen as agent output grows faster than human review capacity" (§3, §13).
- **Reason.** Depends on the evolution of AI and human review; not a law.
- **Impact.** Avoids a falsifiable overclaim in the foundational narrative.

### C8 — "Free forever" → precise standing commitment (D7)
- **Previous.** "The individual daily check is free, forever."
- **Revised.** D7: the **fully local individual core check** remains free and requires no account or payment. (This wording was refined again in the open-source framing pass below; earlier drafts enumerated possible paid layers, which was removed.)
- **Reason.** The intent (never charge for the individual local check, never require an account) is deliberate and within control.
- **Impact.** A clear, reassuring guarantee for an open-source project: the individual local check is free and unrestricted. How value is eventually captured is intentionally outside the scope of this strategy.

### C9 — Normative hierarchy + Standing Decisions register (new §0, §10)
- **Previous.** Decisions repeated across sections as a flat list (D1–D12) with no authoritative home.
- **Revised.** §10 is the **normative source**; each decision has an ID, statement, status, rationale, reversal evidence, and revision date. §0 states that other sections may explain but not exceed §10, and that the card and HTML are **derived** (do not edit independently).
- **Reason.** Repetition across sections is a drift hazard; a single normative register prevents conflicting commitments.
- **Impact.** Any future conflict resolves to §10. Derived files must be regenerated from the Markdown.

### C10 — Consistent label taxonomy applied (§0.2)
- **Previous.** Labels were Decision / Hypothesis / Open / Rejected / Evidence.
- **Revised.** Expanded to the required set: **Current reality, Standing decision, Product requirement, Working hypothesis, Future option, Rejected assumption, Open question, Evidence required.** Applied across all three files and the HTML filter chips.
- **Reason.** The task requires reality, requirement, and future-option to be first-class, distinct labels.
- **Impact.** Every capability-adjacent statement now signals whether it is real, committed, required-but-absent, believed, or gated.

### C11 — New Current Reality section + file (§2, `ARGOT_CURRENT_REALITY.md`)
- **Previous.** No factual inventory; strategy prose was the only description of the product.
- **Revised.** Added a compact reality summary (§2) and a full verified inventory file with a per-capability status, evidence, strategic role, public-claim guidance, and gap.
- **Reason.** The task requires separating reality from aspiration and preventing marketing of aspirations as current.
- **Evidence.** Whole-repo verification (CLI `--help`, `crates/`, `action.yml`, `dist-workspace.toml`, docs).
- **Impact.** A marketing agent now has a single "public claim allowed?" reference.

### C12 — Product gaps captured (`ARGOT_PRODUCT_GAPS.md`)
- **Previous.** Gaps between strategy and product were implicit.
- **Revised.** P0–P3 + Rejected, each gap with reality → desired outcome → strategic reason → North Star step → evidence → relative scope → dependencies → preconditions → success measure.
- **Reason.** The repositioning needs an explicit list of what blocks it.
- **Impact.** P0-1 (accept-time integration) and P0-2 (accept-time signal quality) are named as blockers before the strategy can be executed as marketed.

### C13 — Contributor decision test (new §22)
- **Previous.** No mechanism for a future contributor to self-adjudicate a change.
- **Revised.** Ten representative proposals pre-answered with the governing decision.
- **Reason.** The document must be usable without this conversation.
- **Impact.** Common proposals (optional LLM explainer, required account/cloud, telemetry, "voice" hero, high-FP detector, locking SARIF behind an account, attestation export, removing audit) now have clear, cited verdicts.

### C14 — Benchmark framing caveat recorded
- **Previous.** Headline numbers cited without noting measurement framing.
- **Revised.** `ARGOT_CURRENT_REALITY.md` §2 records that the 0.29% false-alarm figure is the base foreign detector on real history, that a separate CI superset reports higher over-fire under different framing, and that combined accept-time noise across all detectors is unmeasured. Also notes the README arch figure (244/252) trails the data file (264/272).
- **Reason.** "Very low noise" must not be overgeneralized from one detector's headline metric.
- **Evidence.** `landing/src/data/foreign.json`, `benchmarks/latest.json`, `landing/src/data/arch.json`, README.
- **Impact.** Signal-quality claims are scoped honestly; P0-2/P1 name the unmeasured combined noise.

---

## Adversarial consistency review (Part 10 outcomes)

Six reviewers simulated independently; each concern was resolved by a change above or is recorded as
a residual.

- **Product reality reviewer.** Every claim ahead of implementation was relabeled: accept-time auto-run (C1), durable finding history (§2, gap P2-1), retention measurement (C4), "no LLM" (C2), absolute locality (C3). Residual: benchmark framing scoped (C14).
- **Open-source maintainer.** Reviewed commitments for long-term burden. D7 guarantees the individual local check is free and requires no account or payment. Value capture is intentionally out of scope; no mandatory-service or telemetry burden introduced. Derived-file sync is manual and lightweight (no build system added).
- **Privacy reviewer.** D13 makes local-first precise and forbids default telemetry; §8 explicitly refuses to solve the North Star with telemetry and routes measurement to opt-in/local/qualitative. The update-check and model download are documented with their opt-outs.
- **Technical architect.** The "trusted core" ambiguity is resolved by §9 with a concrete per-feature test, and D12 no longer forbids the deterministic encoder the product already uses. D14's "noise threshold" is intentionally left as a value to set, not a blocker.
- **Marketing reviewer.** Checked that nuance did not make the message unusable. The four-layer model (§18) and the do/do-not table (§19) keep concrete, usable language; caveats live in the reality doc and gaps, not in every sentence.
- **Skeptical founder.** Confusions of ambition with evidence were labeled: "retention is the company" (C6), future probabilities (C5), governance-inevitability (rejected, §7/§14). Open questions (§21) are kept, not smoothed away.

**Future-contributor test (10 proposals).** All ten now yield a clear verdict and citation from §10
and §9 (see §22). None required guessing after the changes; where a proposal is gated (team
dashboard, attestation export) the gate is named (§15).

---

## Open-source framing pass (2026-07-22, later the same day)

A language and worldview correction, on top of the hardening pass above. **No strategic conclusion,
product decision, roadmap, or evidence gate changed.** The goal was to describe Argot as what it is —
an **open-source product whose long-term form is deliberately undecided** — and to remove
company/startup framing and move value-capture out of the narrative. The strategy intentionally
remains **agnostic** about how value is eventually captured: it is neither assumed nor rejected, only
placed outside the scope of a document about why the product should exist. Prompted by two
clarifications: (1) value capture is outside the scope of this strategy; (2) the documents should not
read like an open-core roadmap.

Files touched: `FOUNDER.md`, `ARGOT_STRATEGY.md`, `ARGOT_STRATEGY.html`, `ARGOT_STRATEGY_CARD.md`,
and this changelog.

- **Company framing removed.** "the retention engine is the company" → "where Argot's long-term value is created; whether it becomes durable infrastructure." "Every scale of the company" → "every scale of adoption." "company moat" → "moat for the project." "eventual shape of the company" (FOUNDER) → open-source framing.
- **Commercial vocabulary reframed.** "durable business," "highest-EV business," "SaaS arc," "willingness to pay," "monetize / paid layers / hosted services," "enterprise sales/motion/features" were reframed to project/product/adoption/organization-facing language. F2 → "team product (teams adopt it as shared infrastructure)"; F3 → "organization-level accountability / governance."
- **Value capture placed out of scope.** A short standing framing now states plainly: Argot is an open-source product, how value is eventually captured is intentionally outside this strategy's scope (neither assumed nor rejected), the future form is deliberately open, and remaining an independent open-source tool indefinitely is a fully successful outcome (§7, §14 note, card, FOUNDER).
- **Guardrails preserved, not expanded.** The evidence gates (§15), the contributor decision test (§22), and the free-check commitment (D7) keep their protective function; their wording was neutralized (e.g. D7 now "free and requires no account or payment"; the F2 gate now "sustained shared reliance," not "willingness to pay"). These changes soften commercial phrasing without changing which futures are gated or the direction of any decision.
- **Scope note.** `ARGOT_PRODUCT_GAPS.md` (not in this pass's file set) still describes evidence-gated options (its P3 "do not build until gated" list) in their own terms; that file is the internal deferral list and was left unchanged. It can be neutralized on request.
- **Refinement (same day).** Earlier drafts of this pass wrote "no commercialization is intended," which reads as *rejecting* commercialization. That was corrected everywhere to "value capture is intentionally outside the scope of this strategy." The distinction is deliberate: the strategy answers *why the product should exist* and defers *how value is eventually captured* — it neither assumes nor rejects any future (independent forever, foundation stewardship, sponsorship, consulting, hosted services, an organization around the project, acquisition, or something unforeseen).

---

## How to keep these documents in sync

1. Edit `ARGOT_STRATEGY.md` (canonical). Update §10 for any decision change and add a §23 log entry with reversal evidence.
2. Re-sync `ARGOT_STRATEGY_CARD.md` and `ARGOT_STRATEGY.html` by hand from the Markdown. Do not edit them independently.
3. When product reality changes, update `ARGOT_CURRENT_REALITY.md` first, then reconcile any strategy prose that referenced the old reality.
4. Record every substantive change here with before/after/reason/evidence/impact.
