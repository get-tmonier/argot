---
title: Reading the output
description: Anatomy of a hit — severity tiers, sources, the reason, and the evidence line.
group: Guide
order: 4
---

A `check` run groups hits by file. Here's a complete one:

```text
argot check · 2 hunks above threshold (1 foreign · 1 suspicious)
note: argot is a probabilistic style linter — verify before action.

src/utils/http-helpers.ts
  !  L42-L48      8.21  foreign     · workdir · foreign import (import) [a1b2c3d4e5f6]
     ↳ axios — 0 of 47 module specifiers in repo
       common here: react (320×), express (88×), pg (47×)
  42 │ import axios from 'axios';
  43 │
  44 │ export async function fetchUserData(id: string) {

src/api/router.ts
  ?  L102         5.89  suspicious  · staged · rare token sequence (bpe) [b7c8d9e0f1a2]
     ↳ startedAt (0×), _res (3×), use (88×)
  102 │ router.use((req, _res, next) => { req.startedAt = Date.now(); next(); });
```

## Anatomy of a hit

Each hit line carries five things:

- **the marker** (`!` foreign / `?` suspicious / `.` unusual) and **line range** — where the hunk is.
- **the hit hash** (`[a1b2c3d4e5f6]`) — a stable id you can pass to `argot mute`.
- **the score** — the BPE log-likelihood ratio for the hunk. Higher means it diverges more from the
  repo's distribution.
- **the severity tier** — `unusual` / `suspicious` / `foreign` (below).
- **the source** — `workdir`, `staged`, `untracked`, or a commit SHA, so you know where it came from.
- **the reason** that fired — `import` (foreign import), `bpe` (rare token sequence), or `call_receiver`
  (an unfamiliar callee tipped it over). A fourth reason, `convention`, exists in the engine but is
  **off by default** — an internal benchmark-only knob, not something `check` normally emits.

## Severity tiers

Tiers are relative to the calibrated threshold `t` (stored in `.argot/scorer-config.json`):

| Tier | Range | Meaning |
|---|---|---|
| `unusual` | `t ≤ score < t+0.5` | Borderline — worth a glance, don't trust the call |
| `suspicious` | `t+0.5 ≤ score < t+1.5` | Likely worth a look |
| `foreign` | `score ≥ t+1.5` | High-confidence anomaly |

## The evidence line

The `↳` line is the per-hunk evidence — *why* this hunk fired:

- For **bpe** hits it names the surprising identifiers with their repo-wide counts. `startedAt (0×)`
  never appears elsewhere in the repo; `use (88×)` is familiar — the flag is about the *combination*,
  not the words.
- For **foreign-import** and **unfamiliar-callee** hits it shows the offending names plus a
  `common here:` line that orients you to the repo's typical vocabulary in that dimension.

The score and reason are always printed, so a hit is never a black box.

## Files argot stays silent on

argot won't flag **data-dominant files** — modules that are ≥80% top-level array/object literals
(locale tables, fixture arrays, generated lookups). Their string payloads look like foreign
vocabulary, so without this gate the scorer would fire on every line. The same predicate runs at fit
and check time, so the model trains and scores on the same scope.

Test files, config files, and a handful of conventional directories (`docs/`, `examples/`,
`migrations/`, `scripts/`, `build/`, `dist/`) are skipped today as a placeholder default — these will
move to user-configurable rules with the suppression surface.
