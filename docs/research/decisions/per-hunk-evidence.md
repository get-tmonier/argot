# Per-hunk evidence: inline attestation for BPE, slot-comparable orientation for the rest

> **TL;DR.** Argot's check output now shows a `↳` line under each hit
> with reason-specific evidence. For BPE-fired hits the line lists the
> surprising identifiers with their per-token repo attestation
> (`message (1,800×), opts (240×), proposed (5×)`). For import / call-
> receiver hits the line shows the offending names plus a `common here:`
> orientation slice. The per-reason split is deliberate: a single
> "common here" framing read as misdirection on real corpora.

## Context

The first cut of the evidence layer (PR #38, `f834dc6`) gave every hit
a uniform two-line block:

```
↳ <names> — <rarity stat>          # e.g. "0 of 71,811 identifiers in repo"
   common here: <top-N from corpus>
```

The intent was orientation: tell the user what the repo's typical
vocabulary is in the same dimension as the flag. For import hits this
reads as architecture identity:

```
↳ axios — 0 of 47 module specifiers in repo
   common here: react (320×), express (88×), pg (47×)
```

Slot-comparable, useful, lands.

For call-receiver hits the orientation is cluster-scoped (per the
MinHash partitioning the scorer already uses), which is also slot-
comparable.

For BPE hits the framing collapses. On faker the line read:

```
↳ message, opts, proposed (+3 more) — 0 of 71,811 identifiers in repo
   common here: name (3,014×), person (1,210×), de (1,163×)
```

Two failure modes:

1. **The rarity stat misleads.** "0 of 71,811" is technically about the
   BPE *piece sequence*, not the listed identifiers. But every reader
   parses it as "these words don't exist in your repo." On faker
   `message`, `opts`, `proposed` are *all* in the repo — `message`
   appears thousands of times. The line was lying about the words.
2. **`common here:` is non-sequitur.** For a faker-js codebase the
   global top-3 most-frequent identifiers are domain-data words (`name`,
   `person`, locale codes). Showing them next to a flagged test-code
   hunk reads as "you wrote `actual`/`hides`/`wrapper`; the repo
   normally uses `name`/`person`/`de`" — comparing test register to
   product-domain vocabulary, which is structurally wrong. The user
   feedback was direct: "this makes no sense."

## Decision

Drop the BPE `common here:` orientation entirely. Replace both lines
with a single `↳` line carrying per-token repo frequencies on the
flagged identifiers themselves:

```
↳ message (7×), opts (0×), proposed (5×) (+3 more)
```

The reader sees immediately which token is rare and which are familiar.
A genuine novel identifier (truly absent from the repo) renders as
`(0×)` — the zero is the signal.

For import and call-receiver evidence, keep the existing two-line
structure. Their orientation slices are slot-comparable and the rarity
stats are about the right dimension.

## Implementation notes

- `EvidenceCorpus.identifiers` shifted from a top-N `list[CommonEntry]`
  to a full `dict[str, int]` so the BPE collector can look up any
  flagged token, including rare ones that wouldn't fit a top-N slice.
  Calibration JSON grows ~1 MB on faker (71k distinct identifiers);
  acceptable, easily compressible later if it becomes a footprint
  concern.
- `BpeEvidence.surprising_identifiers` is now `list[CommonEntry]`
  (name + repo count). The `rarity` and `common_here` fields are gone.
- The formatter for BPE renders one line; for import / CR it renders
  two. Polymorphic formatters keep per-reason divergence localised so
  Tier 3 ("repo prefers `useEffect` over `componentDidUpdate`") can
  add to one without touching the others.
- Display gate: hits are shown when `flagged=True` (any stage fired
  against the calibrated threshold), not when `bpe_score >= threshold`.
  An import-fired hit on otherwise-quiet code has a tiny BPE-side
  score — gating on it silently hid those hits.

## Consequences

- **Calibration JSON size.** ~25 KB → ~1.4 MB on faker. Bounded by
  distinct identifier count. Optimisable (drop count==1 long tail,
  binary serialisation, gzip) if it bites — not yet.
- **Tier 3 still possible.** The polymorphic formatters and per-reason
  collectors leave the seam open for "alternatives the repo prefers"
  without reshuffling the data shape.
- **No `common here:` for BPE.** Repo-distinctive vocabulary is real
  orientation but not slot-comparable to a flagged hunk's content. If
  Tier 3 lands a substitution-style suggestion ("repo uses X, you
  wrote Y") that goes back on the line; raw frequency stats don't.

## See also

- `.scratch/check-evidence-layer/PRD.md` — the original evidence-layer
  PRD that this decision amends. PRD §"Open questions" flagged the
  BPE-`common here:` quality concern; this branch closes it.
- `engine/argot/scoring/evidence/{types,bpe,formatters}.py`
- Issue [#40](https://github.com/get-tmonier/argot/issues/40) (parent
  evidence ticket)
