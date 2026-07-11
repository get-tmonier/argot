---
title: Reading the output
description: Anatomy of a hit — the rule, confidence tiers, severities, sources, and the evidence line.
group: Guide
order: 7
---

A `check` run groups hits by file. Here's a complete one:

```text
argot check · 2 hunks above threshold (1 foreign · 1 suspicious)
note: argot is a probabilistic style linter — verify before action.

src/utils/http-helpers.ts
  !  L42-L48      8.21  foreign     · workdir · foreign-import [a1b2c3d4e5f6]
     ↳ axios — 0 of 47 module specifiers in repo
       common here: react (320×), express (88×), pg (47×)
  42 │ import axios from 'axios';
  43 │
  44 │ export async function fetchUserData(id: string) {

src/api/router.ts
  ?  L102         5.89  suspicious  · staged · rare-tokens [b7c8d9e0f1a2]
     ↳ startedAt (0×), _res (3×), use (88×)
  102 │ router.use((req, _res, next) => { req.startedAt = Date.now(); next(); });
```

## Anatomy of a hit

Each hit line carries five things:

- **the marker** (`!` foreign / `?` suspicious / `.` unusual) and **line range** — where the hunk is.
- **the hit hash** (`[a1b2c3d4e5f6]`) — a stable id you can pass to `argot mute`.
- **the score** — the BPE log-likelihood ratio for the hunk. Higher means it diverges more from the
  repo's distribution.
- **the confidence tier** — `unusual` / `suspicious` / `foreign` (below).
- **the source** — `workdir`, `staged`, `untracked`, or a commit SHA, so you know where it came from.
- **the rule** that fired — `foreign-import`, `rare-tokens`, or `unfamiliar-callee` from the base
  voice model, `redundant` (a function you already have) and `misplaced` (code in an unusual
  location) from the semantic layer, `layering` (an internal import that crosses a module
  boundary) from the architecture detector, and `test-deleted` / `test-disabled` / `test-weakened`
  (a test removed, skipped, or weakened alongside the production change it covers) from the
  integrity detector. A further rule, `convention`, exists in the engine but rarely fires on its
  own. Every rule defaults to `error` except `test-weakened`, which ships `warn` — it's printed
  like any other hit, but on its own it doesn't fail the check. `argot rules` lists them all with
  their effective severities.

## Confidence tiers

The tier grades **how strong the evidence is** — it drives the marker and the display, never the
exit code (that's the rule's configured *severity* — see
[Configure](/docs/configure/#rules--rule-severities)). For the statistical rules, tiers are
relative to the calibrated threshold `t` (stored in `.argot/scorer-config.json`):

| Tier | Range | Meaning |
|---|---|---|
| `unusual` | `t ≤ score < t+0.5` | Borderline — worth a glance, don't trust the call |
| `suspicious` | `t+0.5 ≤ score < t+1.5` | Likely worth a look |
| `foreign` | `score ≥ t+1.5` | High-confidence anomaly |

`redundant`, `misplaced`, and `layering` findings are always pinned to `unusual`; the integrity
findings (`test-deleted`, `test-disabled`, `test-weakened`) are always pinned to `suspicious`.
None of these carry a score margin — the evidence is an event lookup, not a BPE distance. `argot
check --min-confidence <tier>` filters the display.

## The evidence line

The `↳` line is the per-hunk evidence — *why* this hunk fired:

- For **rare-tokens** hits it names the surprising identifiers with their repo-wide counts. `startedAt (0×)`
  never appears elsewhere in the repo; `use (88×)` is familiar — the flag is about the *combination*,
  not the words.
- For **foreign-import** and **unfamiliar-callee** hits it shows the offending names plus a
  `common here:` line that orients you to the repo's typical vocabulary in that dimension.
- For **redundant** hits it names the existing function the new one duplicates, with its location and
  the similarity — the semantic layer's nearest-code evidence:
  `↳ duplicates slugify (src/utils/text.py:1) — similarity 0.86`.
- For **misplaced** hits it names where the code looks like it belongs:
  `↳ looks like core/downloader code filed under commands/`.
- For **layering** hits it names the established direction the new import breaks:
  `↳ cli → core is this repo's direction — this import reverses it`.
- For **integrity** hits it names the test and what happened to it, plus a note that the
  changeset also touches production source — the co-change requirement that keeps tests-only
  commits (suite curation) silent. Rendered under a `test-disabled` hit at `suspicious`
  confidence, score `1.00`:
  ``↳ test `test_parse` disabled — skip/ignore marker added; this change also modifies parser.py``.

The score and rule are always printed, so a hit is never a black box.

## Machine-readable output

`argot check --format json` and `--format sarif` write **only** the document to stdout (progress and
warnings stay on stderr), so they pipe cleanly. Both are stable contracts. A third machine format,
[`--format github`](#--format-github), emits GitHub Actions workflow commands.

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
| `threshold` | Calibrated threshold the confidence tier is measured against. |
| `confidence` | Evidence strength: `unusual` / `suspicious` / `foreign`. |
| `severity` | The rule's configured severity for this run: `error` (fails the check) or `warn`. |
| `rule` | Stable rule name: `foreign-import`, `unfamiliar-callee`, `rare-tokens`, `convention`, `redundant`, `misplaced`, `layering`, `test-deleted`, `test-disabled`, or `test-weakened` — the same key you'd use in `argot.toml [rules]`, `--rule`, and suppressions. |
| `rule_label` | Human label of `rule` (e.g. `rare token sequence`, `foreign import`). |
| `source` | `workdir` / `staged` / `untracked`, or a short commit SHA. |
| `hash` | Content-based hit hash — paste into `argot mute <hash>`. |
| `evidence` | Rendered evidence lines (empty when the scorer had none). |

### `--format sarif`

SARIF 2.1.0 for code scanning (GitHub `upload-sarif`, etc.). `ruleId` is the rule name
(`foreign-import`, `redundant`, `layering`, …), one SARIF rule per distinct name in
first-appearance order; each result carries the physical location and the raw `score` /
`threshold` / `confidence` / `severity` / `source` / `hash` / `evidence` under `properties`.
Confidence tiers map to SARIF levels — `unusual → note`, `suspicious → warning`,
`foreign → error` — capped at `warning` for a rule configured `warn` (SARIF `error` is reserved
for findings that fail the check).

### `--format github`

GitHub Actions workflow commands — one `::error file=…,line=…::message` (or `::warning` for a
`warn`-severity rule) per hit, which the runner turns into **inline PR annotations** with no upload
step and no extra permissions. Each annotation carries the rule name, score, confidence, evidence,
and the exact `argot mute <hash>` command. Available on `check` and `review`.

The `voice-diff`, `inspect`, `status`, and `list` commands each also emit a stable `--format json`
document — see [The commands](/docs/the-commands/).

## Color

argot colors the confidence markers only when [`NO_COLOR`](https://no-color.org) is **unset** and stdout
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
