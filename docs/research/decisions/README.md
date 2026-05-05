# Decisions

ADR-style writeups for production / architectural choices that aren't
era-driven research. Each doc is self-contained: context, decision, why
not the alternatives, consequences. Reference these when shipping a
change that touches the same surface — and when reverting one, link
the contradicting evidence here rather than silently overriding.

## Index

- **[scope-symmetry.md](scope-symmetry.md)** — calibration scope and
  check-time scope must match. Covers test/config/data/lang/`.history/`
  exclusions, plus the fit-time import-module snapshot that closes the
  "your new foreign import gets absorbed by the model that's about to
  flag it" hole.
- **[per-hunk-evidence.md](per-hunk-evidence.md)** — the `↳` line under
  each hit. Inline per-token attestation for BPE; slot-comparable
  `common here:` orientation for import / call-receiver. Why the
  per-reason split, and why the original uniform framing collapsed on
  real corpora.
- **[asymmetric-calibration-in-prod.md](asymmetric-calibration-in-prod.md)** —
  era-13.5's per-corpus auto-detect of the cluster-rare rule, ported from
  the bench harness into `argot-calibrate` and made the default. The
  +16.6pp recall win on faker-js is now what end users actually see, not
  a bench-only feature.
- **[per-language-calibration.md](per-language-calibration.md)** —
  per-language calibration end-to-end (v2 `scorer-config.json` keyed by
  language, dispatch by file extension at score time), engine streaming
  refactor that lets the scorer scale to monorepo-class corpora without
  sampling, and the bench wall-time tuning (single outer seed,
  `threshold_n_seeds=3`, per-PR extract `--limit`) that ships alongside.
  Dagster pinned as the reference multi-language corpus.

## When to write one

If the change you're shipping satisfies any of these, write a decision
doc:

- Crosses a layer boundary (engine ↔ CLI, calibration ↔ check, scoring
  ↔ rendering) and the rationale would be lost in a single commit msg.
- Locks in behaviour that's plausibly worth revisiting (e.g. "we drop
  test files today; here's the design hook for `.argotignore`").
- Resolves a tension between two competing approaches and the
  alternative is worth recording for future-you.

If the change is purely additive / contained / obvious, just commit it.
ADRs that say "we wrote some code" are noise.
