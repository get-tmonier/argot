# Argot — Founder Manifesto

*One page to regain strategic clarity. Read it in two minutes; operate from it for a month.*

> This file is an operating manifesto. If it conflicts with the canonical strategy or
> current-reality documents, those documents win.

## Argot exists because

For most of software history, understanding code was a byproduct of writing it. You could not
produce code without holding a model of it in your head, and reading was how that model passed
between people. AI weakened that link. Developers now accept responsibility for code they did not
write and did not fully read, and better models make this more common, not less: more code, more
trust, less reading. Argot exists to give a developer awareness at the moment they accept that
code. This is not "AI makes mistakes." The code is often correct. The problem is that no human
understood it, and someone still has to stand behind it.

## What Argot does

Argot is a repository-grounded check. It shows what an agent introduced that deserves attention,
judged against the repository's own history and structure rather than another generative model's
opinion. Its findings are evidence you can reproduce, not a review essay.

Be precise about what exists today:

- `argot audit` is real and is the zero-setup front door: run it on your own repo and see what agents have already introduced.
- The detectors and `argot check` are real.
- Automatic checking at the moment you accept an agent's diff is the experience we want, but it is **not yet fully shipped**. Today the only automatic piece is a pre-write guardrail; the commit-time check is run by hand, by a skill, or through a hook you set up. Do not describe accept-time checking as if it already runs itself.

## The operating model

> Audit installs you. Check-on-accept keeps you.

Memorable catches — an agent that weakened a test or routed around a check — earn attention.
Frequent awareness — a foreign dependency, a reinvented helper, misplaced code — earns the habit.
These are two engines doing two jobs. Acquisition without habit is a viral toy; habit without
acquisition is a tool nobody hears about. Keep both. Let neither stand in for the other.

## The North Star

> Audit-to-habit conversion.

The funnel we care about:

1. A developer runs `argot audit`.
2. They install or configure Argot.
3. They enable recurring checks in their agent workflow.
4. They still trust and use Argot after 30 days.

This is currently a **conceptual** North Star. Argot has no default telemetry and cannot directly
observe 30-day retention, and we will not add default telemetry to see it. We read retention
through proxies, opt-in local signals, and talking to users.

## What we optimize for

Signal quality above all. Low noise. Speed measured in seconds. Evidence over opinion. Local-first
operation. A genuinely useful individual developer experience. Integration into a behavior
developers already have, not a new ritual. Embeddability. Honest capability claims. Retained trust,
not stars alone.

## What we refuse to become

- **A generic AI reviewer.** Our findings are reproducible evidence, not a model's guess.
- **A prose-generating review bot.** A second opinion machine is not what the moment needs.
- **Merely a style or "voice" linter.** Style is the part models fix fastest; that is not the durable job.
- **A generic SAST replacement.** We sit after those tools and answer a different question.
- **An "AI governance platform" marketed before anyone asks.** That bets the brand on the least likely future.
- **A cloud service required to read local code.** The core must run on your machine.
- **A pile of detectors optimized for feature count.** More detectors, more noise, less trust.
- **A tool that paywalls the individual local check.** That check is the foundation of everything.
- **A product whose authoritative findings are decided by a generative model.** The core stays deterministic and inspectable.

None of this is about competitors. It is about knowing what we are.

## Non-negotiables

- Signal quality is existential.
- Noise destroys the habit.
- The local individual core check stays free.
- No account or cloud is required for the core.
- No default telemetry.
- No generative or opinion-forming model in the authoritative analytical path. A local, deterministic code encoder is part of the core and is fine.
- Configuration stays portable and user-owned.
- "Voice" may live in the brand; it never carries the product explanation.
- Never market a product requirement as current reality.
- No platform or governance work before its evidence gate is crossed.

## Strategic posture

> Conviction on the foundation. Options on the destination.

Argot is an open-source product. Its identity is not determined by how it is funded, and how value
might eventually be sustained or captured is intentionally outside the scope of this strategy —
neither assumed nor rejected. Its long-term form is deliberately left open: it might stay a
standalone developer tool, grow into something teams rely on together, or take a shape we cannot yet
predict. None of that is decided, and none of it is the point. The point is whether the project
becomes genuinely useful. Remaining an independent open-source tool forever is a complete success. We
do not build a speculative branch before the evidence for it appears.

## Founder decision heuristics

- When in doubt, prefer signal quality over detector count.
- Prefer habit over launch virality.
- Prefer evidence over adjective.
- Prefer a real workflow over a new ritual.
- Prefer local and portable over required infrastructure.
- Prefer an honest limitation over an impressive false claim.
- Before adding a finding, ask: would I genuinely want to know this before accepting the diff?
- Before adding a feature, ask: does this strengthen the shared foundation, or am I prematurely building one possible future?
- Before changing messaging, check current product reality first.
- If a change makes Argot noisier, it carries the burden of proof.

## Final founder statement

We are committed to one thing: making the daily check good enough that developers genuinely keep it
enabled. Argot is an open-source product, and we are intentionally uncommitted about how value might
eventually be sustained or captured. The project may remain independent forever, or it may evolve in
ways we cannot yet predict; that question is deliberately deferred until reality provides evidence.
The next real answer will come from users, not from another strategy exercise, so we ship a check
worth keeping, watch what people do with it, and let that decide the rest.

## Canonical references

- [Strategy and positioning](./docs/strategy/ARGOT_STRATEGY.md)
- [Current product reality](./docs/strategy/ARGOT_CURRENT_REALITY.md)
- [Product gaps](./docs/strategy/ARGOT_PRODUCT_GAPS.md)
- [Strategy card](./docs/strategy/ARGOT_STRATEGY_CARD.md)
- [Strategy changelog](./docs/strategy/ARGOT_STRATEGY_CHANGELOG.md)
- [Human-readable strategy](./docs/strategy/ARGOT_STRATEGY.html)
