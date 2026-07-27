---
name: argot-check
description: Score your working changes with argot — flag code foreign to this repo's own patterns (unfamiliar dependencies, APIs, constructs), functions the repo already has, code filed in the wrong place, imports that break the repo's layering, and tests weakened, disabled, or deleted alongside a production change — before committing. Use after generating or editing code, before a commit, or when the user asks "check my changes with argot", "is this in-voice", or "does this match how we write code here".
---

# argot-check

Run `argot` on the current changes and act on what it reports. argot is
statistical; false positives happen. Every hit names a **rule**, and the rule —
not the confidence glyph — tells you what to do. The human has the last word.

## Preconditions

1. `argot --version` — if missing, tell the user how to install it (see
   <https://argot.tmonier.com/docs/getting-started/>) and stop.
2. The repo must be fitted: `.argot/scorer-config.json` exists. If not, run the
   **argot-setup** skill (or `argot init`) first, then continue.

## Run it

Score the changes you care about, as JSON:

```
argot check --format json            # working-tree changes
argot check --staged --format json   # what's about to be committed
```

Exit codes: `0` clean (or warn-severity hits only) · `1` at least one
error-severity hit · `2` setup/usage error. **Treat `1` as "there is something
to act on," not as a mystery failure** — walk the decision tree below.

Each hit in the JSON `hits` array carries:

| Field | Use |
|---|---|
| `rule` | Kebab-case rule name (`foreign-import`, `redundant`, …) — **branch on this** (see the rules table and decision tree). |
| `rule_label` | Human label of the rule: `foreign import`, `already implemented here`, … |
| `severity` | `error` or `warn` — the rule's configured severity for this run. Error hits drive exit code `1`. |
| `confidence` | `unusual` / `suspicious` / `foreign` — strength of the evidence, display-grade only (see below). |
| `evidence` | The lines to show the user — names the foreign symbol, the duplicated function, or the area the code belongs in. |
| `hash` | Stable id for `argot mute <hash>`. |
| `path`, `line_start`, `line_end` | Where it is. |
| `source` | `workdir` / `staged` / `untracked` / a commit SHA — where the change came from. |
| `score`, `threshold` | Raw internals. **Read `rule` and `severity`, not these** — they sit on different scales per signal, so comparing them directly is meaningless. |

In `--format human`, each hit's meta line reads
`!  L1-L10  1.00  foreign  · staged · foreign-import [a1b2c3d4]` — confidence
glyph and tier, then the source and the rule name.

`--format` accepts `human` / `json` / `sarif` / `github` (`github` emits
workflow commands for inline PR annotations). `--min-confidence` filters what
is *displayed* (it does not change the exit code); `--quiet` silences
informational stderr.

## The rules

Twelve built-in rules in five groups (plus any repo-local custom rules, group `custom`). `rule-tampered` (group `governance`) fires when the change itself weakens a locked rule — treat it as the highest-priority finding: it means the diff touched the guardrail, not the code. `argot rules` prints this registry with the
repo's effective severities.

| Rule | Group | What it means | What to do |
|---|---|---|---|
| `foreign-import` | voice | An import of a dependency the repo has never used. | Read the evidence — it names the import and what the repo reaches for instead. Switch to the in-voice dependency unless the new one is deliberate. |
| `unfamiliar-callee` | voice | A call to a receiver or callee the repo's code never calls. | Check whether the API is wanted; prefer the API the repo already uses. |
| `rare-tokens` | voice | A token sequence statistically foreign to the repo's voice. | Read the hunk; if it's an off-voice idiom, rewrite it with the repo's vocabulary. |
| `convention` | voice | A construction that breaks a convention learned from the repo. | Follow the convention named in the evidence, or justify the exception. |
| `superseded` | voice | New code uses a pattern the repo has replaced — mined from its history, or declared in `argot.toml`. Warn by default. | Use the replacement named in the evidence; only fails the check under `--error-on-warnings`. |
| `redundant` | semantic | A new function that duplicates one the repo already has. The evidence `↳ duplicates <symbol> (<path>:<line>) — similarity 0.XX` names the original. | **Do not ignore.** Open the cited file, compare, and call the existing function instead of keeping the reimplementation — or justify and mute with a reason. |
| `misplaced` | semantic | A function that looks like it belongs in another module area. The evidence reads `↳ looks like <area> code filed under <area>`. | Propose moving the code to the cited area, or justify the placement. |
| `layering` | architecture | An internal import that reverses the repo's established layering direction. | Don't introduce the import — invert the dependency or route through the intended layer. |
| `test-deleted` | integrity | A test (or whole test file) removed while the production code it exercised still exists. | Restore the test or explain why it's obsolete; if the deletion is legitimate (feature removed), the code that exercised it should be gone too. |
| `test-disabled` | integrity | A skip/ignore marker added, or a test gutted, while production code changes. | Un-skip and fix the code, or record why the skip is temporary; skipping to make a failing suite green is the exact behavior this rule exists to catch. |
| `test-weakened` | integrity | Assertions removed, tautologized, or loosened while production code changes. | Restore the assertion strength; if the expected value legitimately changed, say why in the commit/PR rather than silently retargeting. |

A repo may also carry its own rules under `.argot/rules/<name>/` — a fifth group, `custom`.
Treat a `custom`-group hit exactly like any row above: read the rule's `evidence`, act on it, or
justify and mute. `argot rules` lists every custom rule alongside the built-ins, with its source
directory, so a quick look there tells you the repo's full vocabulary before you triage a hit you
don't recognize.

## Confidence is evidence strength, not priority

`foreign` (`!`) / `suspicious` (`?`) / `unusual` (`.`) grade **how strong the
statistical evidence is** — nothing more. They are display-only: they never
drive the exit code (severities do), and `--min-confidence` only filters the
display. `redundant`, `misplaced`, and `layering` are always reported at
`unusual` because they come from a single retrieval/graph signal rather than a
calibrated score. The three `integrity` rules (`test-deleted`, `test-disabled`,
`test-weakened`) pin to `suspicious` — each is a discrete, evidenced event (a
marker added, assertions excised), stronger than `unusual` but not the
categorical certainty of a 0-usage import. **An `unusual` hit is NOT "usually
fine" — look at its rule.** An `unusual` `redundant` hit still means the repo
already has that function.

## Severities and configuration

Every rule defaults to `error` except `test-weakened` and `superseded`, which
ship `warn` (error → exit `1`; warn → shown, exit `0`;
off → silent). Configure durably in `argot.toml`:

```toml
[rules]
misplaced = "warn"     # one rule
semantic = "off"       # or a whole group: voice / semantic / architecture / integrity
```

or per run with `argot check --rule <name|group>=<severity>`. In strict CI,
`--error-on-warnings` makes warn hits fail the run too.

## Gauge trust first

Run `argot inspect` and read the verdict. If it's **Not recommended**, the
statistical voice model isn't well-calibrated on this repo — down-weight the
`voice`-group hits accordingly and say so. **Ready — with notes** is usable
as-is; the notes say what to keep an eye on.

## What a hit means — and what a clean run doesn't

argot reliably flags a **novel pattern** foreign to this repo — a dependency it
has never imported, an API it never calls, or a whole paradigm (a Django-style
view in a FastAPI repo, a different HTTP client, hand-rolled validation) it
never writes. When the foreign symbol is in the change, it catches ~98% of
these. Trust a `foreign-import` hit. The `semantic` and `architecture` rules
add duplicated functions, misfiled code, and layering breaks on top; the
`integrity` rules add a test weakened, disabled, or deleted alongside the
production change it covers.

It does **not** catch every *in-vocabulary* break — where every token is
already in the repo and only the choice is wrong. So **a clean run means "none
of the configured rules fired," not "this matches every convention."** Don't present
a clean argot result as a guarantee the code is idiomatic — it's silent on some
of the subtle stuff by design.

## Decision tree — branch on the rule

For each hit in the JSON, branch on `rule`:

- **`foreign-import` / `unfamiliar-callee` / `rare-tokens` / `convention`** —
  the change uses a dependency, API, or idiom foreign to the repo. Read the
  evidence line — it shows the surprising identifier and what the repo uses
  instead. Ask: *does this match how the repo already does this?*
  - If a well-established in-voice option exists, rewrite your change to use
    it (or suggest the switch if the code isn't yours).
  - If the choice is deliberate, tell the user they can record it:
    `argot mute <hash> --reason "…"`.
- **`superseded`** — the change uses a pattern this repo has moved on from
  (mined from history, or declared in `argot.toml`). Read the evidence — it
  names the replacement and the commits that made the switch (or the
  declared reason). It's warn by default: report it and recommend the named
  replacement; it only fails the check under `--error-on-warnings`. `argot
  conventions` lists every migration still in progress, with the leftover
  files the refactor hasn't reached yet.
- **`redundant`** — do **not** ignore this. Open the file cited in the
  evidence (`↳ duplicates <symbol> (<path>:<line>)`), compare the two
  functions, and **use the existing one** instead of the reimplementation. If
  the duplication is genuinely intentional (e.g. a deliberate fork), justify
  it and mute with that reason.
- **`misplaced`** — propose moving the code to the area the evidence cites, or
  explain to the user why this location is right (then mute with that reason).
- **`layering`** — don't introduce this import. Invert the dependency or go
  through the layer the repo's architecture intends. Only mute if the user
  confirms the layering is deliberately changing.
- **`test-deleted`** — restore the test or explain why it's obsolete; if the
  deletion is legitimate (feature removed), the code that exercised it should
  be gone too.
- **`test-disabled`** — un-skip and fix the code, or record why the skip is
  temporary; skipping to make a failing suite green is the exact behavior this
  rule exists to catch.
- **`test-weakened`** — restore the assertion strength; if the expected value
  legitimately changed, say why in the commit/PR rather than silently
  retargeting.

## Report format

Give a short, calm summary. For example:

```
argot: 2 hits in your changes (1 error · 1 warn)

! src/http.ts:42    foreign-import — axios; 0 of 47 imports here use it
                    the repo reaches for: node-fetch (88×)
                    → switched to node-fetch (or record it: argot mute a1b2c3d4 --reason "…")
. src/user.ts:10    redundant — duplicates slugify (src/utils/text.ts:14) — similarity 0.93
                    → call the existing slugify instead of the new copy
```

## Suppressions

- One line, in the code:
  `# argot: ignore-next-line rule=<name|group> — reason`
- Per hit: `argot mute <hash> --reason "…"` — a committed `[[mute]]` in
  `argot.toml` covering **that hit alone**. The same finding in a sibling file
  has its own hash and stays flagged.
- Standing: `argot mute --path '<glob>' --rule <name|group> --reason "…"` —
  covers every future hit under the glob. Reach for this when the decision is
  about a tree; a hash mute per file is the failure mode it exists to avoid.
- Housekeeping: `argot list-mutes` shows every active suppression across all
  three surfaces; `argot review-mutes` re-scores muted files and reports which
  suppressions no longer fire (`--prune` removes the dead ones).

## Hard rules

- **Fix or justify — don't silently ignore.** An error-severity hit fails
  `argot check`; resolve it by fixing the code you wrote or by offering the
  user the mute command with a real reason.
- **Never rewrite pre-existing code the user wrote** just to silence a hit.
  For code you authored in this session, applying the rule's fix is the job.
- **Never mute on the user's behalf** without a real reason they'd endorse.
  Muting is a human decision; offer the exact command instead.
- False positives happen. If the user says a hit is fine, that's the end of
  it — offer to mute it with their reason so it doesn't come back.

## If the CLI and this document disagree

If the binary reports a rule not covered by this document, trust the binary:
run `argot rules` for the registry and `argot <cmd> --help` — the CLI is the
source of truth, this skill may lag behind it.
