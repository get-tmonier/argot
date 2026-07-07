---
title: Reading the output
description: Anatomy of a hit — severity tiers, sources, the reason, and the evidence line.
group: Guide
order: 6
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
  (an unfamiliar callee tipped it over) from the base voice model, plus `redundant` (a function you
  already have) and `misplaced` (code in an unusual location) from the semantic layer. A further
  reason, `convention`, exists in the engine but is **off by default** — an internal benchmark-only
  knob, not something `check` normally emits.

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
- For **redundant** hits it names the existing function the new one duplicates, with its location and
  the similarity — the semantic layer's nearest-code evidence:
  `↳ duplicates slugify (src/utils/text.py:1) — similarity 0.86`.
- For **misplaced** hits it names where the code looks like it belongs:
  `↳ looks like core/downloader code filed under commands/`.

The score and reason are always printed, so a hit is never a black box.

## Machine-readable output

`argot check --format json` and `--format sarif` write **only** the document to stdout (progress and
warnings stay on stderr), so they pipe cleanly. Both are stable contracts.

### `--format json`

argot's own schema. The top-level object carries:

| Field | Meaning |
|---|---|
| `tool` | `{ name, version }` — the argot build. |
| `model` | Combined fingerprint of the fitted model that scored the diff. |
| `repo` | Repository path, as passed on the CLI. |
| `scanned` | Human label of what was scored (e.g. `workdir`, `3 commit(s) (a..b)`). |
| `hunks_scanned` | Total hunks scored, including below-threshold ones. |
| `files_scanned` | Per-file `{ path, hunks }` — a denominator per file. |
| `hits` | The above-threshold hits (below). |

Each entry in `hits[]`:

| Field | Meaning |
|---|---|
| `path` | Repo-relative, `/`-separated file path. |
| `line_start`, `line_end` | 1-based hunk line range. |
| `score` | The hunk's score. |
| `threshold` | Calibrated threshold the severity tier is measured against. |
| `severity` | `unusual` / `suspicious` / `foreign`. |
| `reason` | Scorer reason code: `bpe`, `import`, `call_receiver` (base voice model), `redundant`, or `misplaced` (semantic layer). |
| `reason_label` | Human label of `reason` (e.g. `rare token sequence`, `foreign import`). |
| `source` | `workdir` / `staged` / `untracked`, or a short commit SHA. |
| `hash` | Content-based hit hash — paste into `argot mute <hash>`. |
| `evidence` | Rendered evidence lines (empty when the scorer had none). |

### `--format sarif`

SARIF 2.1.0 for code scanning (GitHub `upload-sarif`, etc.). One rule per distinct `reason` code —
including the semantic layer's `redundant` and `misplaced`, which get their own auto-generated rules;
each result carries the physical location and the raw `score` / `threshold` / `severity` / `source` /
`hash` / `evidence` under `properties`. Severity tiers map to SARIF levels: `unusual → note`,
`suspicious → warning`, `foreign → error`.

The `voice-diff`, `inspect`, `status`, and `list` commands each also emit a stable `--format json`
document — see [The commands](/docs/the-commands/).

## Color

argot colors the severity markers only when [`NO_COLOR`](https://no-color.org) is **unset** and stdout
is a terminal. Set `NO_COLOR=1`, or redirect stdout to a file/pipe, for plain text. The machine
formats above are never colored.

## Files argot stays silent on

argot won't flag **data-dominant files** — modules that are ≥80% top-level array/object literals
(locale tables, fixture arrays, generated lookups). Their string payloads look like foreign
vocabulary, so without this gate the scorer would fire on every line. The same predicate runs at fit
and check time, so the model trains and scores on the same scope.

Test files, configuration files, and a set of conventional directories (`tests/`, `docs/`, `examples/`,
`migrations/`, `build/`, `dist/`, and more) are skipped by the built-in **`argot:recommended`** set,
and any file argot detects as auto-generated or data-dominant is skipped structurally. All of it is
yours to change — see [Configure](/docs/configure/) for `argot.toml`, inline comments, and mutes.
