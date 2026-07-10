---
title: Configure
description: One committed argot.toml controls everything — what argot excludes ([exclude]), how it spots generated/data files ([detect]), each rule's severity ([rules]), and the hits you've accepted ([[mute]]) — plus inline comments and a gitignored argot.local.toml for personal overrides.
group: Guide
order: 5
---

argot is a statistical linter: it will sometimes learn from the wrong files, or
flag a line you meant to write. Every one of those is fixable, and every fix is
plain text you commit alongside your code — in **one file, `argot.toml`**.

`argot init` writes an `argot.toml` at your repo root with the effective
defaults spelled out, so nothing is hidden. It has four sections:

- **`[exclude]`** — *what argot learns from and looks at.* The built-in
  recommended pattern list (editable) plus your own path patterns.
- **`[detect]`** — *how argot spots code it shouldn't learn from:* the
  generated-file comment markers and the data-dominance threshold, now visible
  and editable.
- **`[rules]`** — *how loud each rule is:* `error`, `warn`, or `off`, per rule
  or per group.
- **`[[mute]]`** — *specific hits you've judged fine,* a durable audit trail.

Inline `# argot: ignore` comments are a fourth surface (per-line, in the source
itself), and a gitignored **`argot.local.toml`** lets any one dev layer personal
overrides on top. A [reference table](#which-files-argot-writes-and-where) at the
end lists every file argot writes and whether it's committed.

## `[exclude]` — set the scope

**Location:** the `[exclude]` section of `<repo-root>/argot.toml`. Commit it so
the whole team and CI share the same scope.

`[exclude]` has **two** gitignore-style pattern lists that differ only in how a
match is treated:

```toml
[exclude]
# The built-in argot:recommended set — dropped SILENTLY (as if absent). init
# writes the full list; edit it freely (see below).
recommended = [
  "test*/", "docs/", "build/", "dist/",   # … and the rest of the defaults
  "conftest.py", "*.test.*", "*.config.*",
]

# Your repo-specific excludes — still SCORED, but their hits are dropped and
# counted on stderr, so every exclusion stays auditable. One pattern per entry,
# each with a trailing `# reason` comment.
paths = [
  "legacy",                 # bare name → a dir/file named `legacy` at ANY depth
  "vendor/",                # trailing slash → that directory only, never a `vendor` file
  "/generated.py",          # leading slash → anchored to the repo root (this file only)
  "src/proto/",             # an interior slash also anchors to the root
  "src/*.gen.ts",           # `*` matches within ONE segment (does not cross `/`)
  "**/snapshots/**",        # `**` spans directories, at any depth
  "*.min.js",               # bundled/minified output, anywhere
  "!keep.min.js",           # `!` re-includes a path an earlier pattern excluded (last match wins)
]
```

The pattern rules (shared by both lists), precisely:

- `*`, `?`, and `[…]` match within a path segment; `**` spans directories.
- A leading `/` — or **any interior `/`** — anchors the pattern to the repo root;
  a bare name (`legacy`) matches at any depth.
- A trailing `/` (`vendor/`) matches a directory and everything under it, never a
  same-named file.
- A leading `!` re-includes a path an earlier pattern excluded — **last match
  wins.**

Don't hand-guess the directories to exclude — [`argot init --suggest`](/docs/setup/)
surfaces the generated/data-heavy ones with evidence, and an agent can name the
vendored/legacy ones from your tree. See [Setup](/docs/setup/).

### Editing the `recommended` set

`init` writes the full built-in `argot:recommended` set into `recommended` — the
directories and files that are almost never part of a repo's authored voice:

- **Directories** (matched at any depth): `test*/` (any dir starting `test`),
  `__tests__/`, `doc/` `docs/` `example/` `examples/` `migration/` `migrations/`
  `benchmark/` `benchmarks/` `fixtures/` `scripts/` `build/` `dist/`
  `__pycache__/` `.git/` `.history/` `.tox/` `.eggs/`.
- **Files:** `test_*` and `conftest.py`; `*.test.*` / `*.spec.*` / `*.config.*`
  (so `x.test.ts`, `vite.config.ts`); and dotfile `.*rc.*` configs (`.babelrc.js`).

Because it's a plain list, you tune it **per entry**: remove `"test*/"` to learn
from your tests, add `"vendor/"` to drop a vendored tree silently, or set
`recommended = []` to disable the set entirely and rely on `paths` alone.
(Detecting generated and data files is a separate concern — see `[detect]` next —
and stays on regardless.)

## `[detect]` — how argot spots generated & data files

**Location:** the `[detect]` section of `argot.toml`. These were hardcoded
heuristics; now they're yours to tune. `argot init` writes today's defaults out
in full.

```toml
[detect]
# Share of a file's non-blank lines that must be static data literals before the
# file counts as data, not authored voice (locale tables, fixture arrays).
data-threshold = 0.65

# A file whose head comments contain any of these (case-insensitive) is treated
# as generated and kept out of the voice model. Trim ones you don't want, or add
# your own codegen banner.
generated-markers = [
  "@generated",
  "auto-generated",
  "do not edit",
  "generated by protoc",
  "generated by swagger",
  "generated by openapi",
  # … init writes the full default list (protobuf/gRPC, OpenAPI, ORMs,
  #    binding/codegen tools, bison/flex/moc, and more) …
]
```

The same resolved `[exclude]` + `[detect]` govern what argot **learns** from,
**calibrates** on, and **checks** — they never drift apart.

- **`generated-markers`** are scanned only in a file's *head comments*, so an
  "auto-generated" phrase inside a docstring or mid-file string can't trip a
  false positive. Set it to `[]` to turn comment-based generated detection off.
- **`data-threshold`** is file-level; a per-row value-dominance test (a fixed
  internal 0.80) still skips individual data-literal rows inside an otherwise
  code file. Set `data-threshold = 1.0` to effectively disable file-level data
  detection.
- Go, Rust, and C# generated files are *also* caught by language-**structural**
  tells built into argot (Go's `// Code generated … DO NOT EDIT.`, Rust's
  `@generated`, C#'s `<auto-generated>` / `[GeneratedCode]`) — those aren't comment
  phrases, so they're not in this list and aren't disabled by emptying it. (C#
  honors your `generated-markers` on top of its structural tells; Go and Rust
  are purely structural.)

## `[rules]` — rule severities

**Location:** the `[rules]` section of `argot.toml`. Every finding argot emits
belongs to one of **seven stable rules**, in three groups:

| Group | Rules |
|---|---|
| `voice` | `foreign-import` · `unfamiliar-callee` · `rare-tokens` · `convention` |
| `semantic` | `redundant` · `misplaced` |
| `architecture` | `layering` |

Each rule carries a **severity**: `error` (reported, fails `argot check` with
exit 1), `warn` (reported, does not fail the check), or `off` (the rule does not
run). **Everything defaults to `error`** — argot gates on all of it until you
say otherwise. Keys are rule names or whole group names; a rule-specific entry
always beats its group entry:

```toml
[rules]
misplaced = "warn"    # report placement findings, but don't fail the check
semantic  = "off"     # …or disable the whole embedding-based group
                      # (with the group off, fit/check skip the model download
                      #  and never build or load the semantic index)
```

Precedence, ascending: built-in defaults → `argot.toml` → `argot.local.toml` →
CLI (`argot check --rule <name|group>=<severity>`, repeatable). An unknown rule
name or severity is a warning on stderr, never a failed run.

Two companions on the CLI:

- **`argot rules`** (`--format json` for machines) lists every rule with its
  group, effective severity for this repo, and a one-line description.
- **`argot check --error-on-warnings`** makes `warn`-severity findings fail the
  check too — strict-CI mode without touching the committed config.

Severity is about the **exit code**. It's distinct from the *confidence* tier
(`unusual` / `suspicious` / `foreign`) a hit displays with, which grades the
evidence — see [Reading the output](/docs/reading-the-output/).

## Inline comments — mute one line

**Location:** in the source file itself, on the line above the code (or around
the block) you're excusing. The comment token is the language's own line comment:
**`#` for Python and Ruby, `//` for everything else** (TypeScript/JavaScript, Go,
Rust, C, C++, Java, C#, PHP) — the language adapter supplies it.

When an exception is deliberate, say so where it lives. This block uses every
form:

```python
# argot: ignore-next-line — vendored oddity we keep on purpose
weird_call()

# argot: ignore-next-line rule=foreign-import — deliberate one-off dependency
import boto3

# argot: ignore-block-start — legacy shim, do not judge this region
legacy_thing()
more_legacy()
# argot: ignore-block-end
```

Adopting argot on an existing codebase with a wall of findings? `argot check
--add-ignores` inserts one of these comments above every current finding
(tagged `baselined by --add-ignores; review`) so the first run goes green and
each acceptance stays a greppable, reviewable line — the same move as
`ruff --add-noqa`.

The `//` languages are identical, just with a different token:

```ts
// argot: ignore-next-line rule=rare-tokens - generated glue, keep as-is
oddPhrasing();
```

The rules:

- **A reason is mandatory.** A suppression comment without one is reported as a
  warning and ignored (it's also the note your future self reads). Separate the
  reason from the directive with `—`, `-`, or `:`.
- **`rule=<name>`** (optional) scopes the mute to one rule (`foreign-import`,
  `rare-tokens`, `unfamiliar-callee`, `convention`, `redundant`, `misplaced`,
  `layering`) or a whole group (`voice`, `semantic`, `architecture`). Omit it to
  mute the hunk whatever fired. An unknown rule name is a warning, and the
  directive is ignored.
- **`ignore-next-line`** mutes the single line below the comment.
  **`ignore-block-start` … `ignore-block-end`** mute everything between them
  (the `-end` needs no reason). An unclosed block suppresses to end of file with
  a warning.

## `[[mute]]` — accept a specific hit

**Location:** the `[[mute]]` tables in `argot.toml`. `argot mute` appends one;
you can also hand-edit it. It's **committed**, so a mute is a shared, reviewable
audit trail that a teammate and CI inherit.

Every hit prints a stable `[hash]`. To accept it for good, mute the hash:

```text
argot mute a1b2c3d4e5f6 --reason "adopting axios repo-wide"
argot mute a1b2c3d4e5f6 --reason "temporary shim" --expires 30d
```

`--reason` records why (recommended everywhere argot reports a hit); `--expires`
takes a **day count** (`30d`, or a bare `30`), which argot resolves to a calendar
date in the file. `mute` reads the last `check` run to learn which file the hash
belongs to, so run `argot check` first. The append is a format-preserving edit,
so your hand-written sections and comments are never rewritten.

Review and prune what you've muted:

```text
argot list-mutes            # every active suppression, across all surfaces
argot review-mutes          # report hash-scoped mutes whose file is gone
argot review-mutes --prune  # …and rewrite the [[mute]] tables to drop the dead ones
```

`review-mutes` flags a hash-scoped mute as **dead** once the file it names no
longer exists (in the working tree or `HEAD`) — the point at which it can never
fire again. `--prune` drops only those; a mute guarding a file you still have is
always kept.

### The `[[mute]]` format

Each `[[mute]]` is a TOML table. This example shows every field a rule can carry:

```toml
[[mute]]
path = "src/vendored/**"     # REQUIRED — glob (fnmatch; `*` crosses `/`)
rule = "rare-tokens"         # optional — a rule or group name (e.g. "semantic"); scopes the mute to that signal
hash = "a1b2c3d4e5f6"        # optional — pin to one specific hit (argot mute writes this)
expires = "2026-12-31"       # optional — YYYY-MM-DD; ignored ON/AFTER this date
reason = "vendored upstream" # REQUIRED — why (surfaces in list-mutes and code review)

# A hand-written rule needs only path + reason — it then covers every hit under the glob:
[[mute]]
path = "generated/**"
reason = "protobuf stubs, never our voice"
```

Only `path` and `reason` are mandatory. An entry with a missing/invalid field (no
`path`, no `reason`, an unknown `rule`, a malformed `expires`) is skipped with a
warning on stderr — one bad entry never voids the file. An entry with `expires`
in the past is treated as expired and ignored; one expiring **today** is still
active.

## `argot.local.toml` — personal overrides

Want a scratch directory excluded on *your* machine only, without touching the
committed config? Put it in **`argot.local.toml`** at the repo root — `init`
gitignores it for you. It deep-merges over `argot.toml`: scalars there win,
list entries (`paths`, `generated-markers`, `[[mute]]`) append, and `[rules]`
entries override the committed ones key by key (the CLI's `--rule` still beats
both).

```toml
# argot.local.toml — gitignored, personal, uncommitted.
[exclude]
paths = ["scratch/", "experiments/"]   # appended to the committed excludes
```

## `[fit]` — the background auto-refresh

A fit is a snapshot: as the repo merges new dependencies and modules, a stale
model starts reading your own accepted code as foreign. argot keeps itself
fresh — when a `check` notices the fit is **10+ commits behind HEAD** (or more
than a week old with any drift), it refits **in the background**, detached, at
most once a day, and says so in one dim line. The check you just ran used the
old model (zero added latency); the next one scores against the fresh voice.
The refit reads committed HEAD — never your working tree — and the semantic
index reuses the embeddings of unchanged functions, so a routine refresh costs
seconds. CI never auto-refits (the Action refits per base advance). To opt out
and drive `argot fit` yourself:

```toml
[fit]
auto-refresh = false
```

Freshness is separate from **calibration drift** — a new `gen/` dir or a
vendored SDK appearing in the tree, or an `argot.toml` edit the model hasn't
absorbed yet. argot watches for both itself, and you never have to guess when
to recalibrate:

- every `fit`/`init` re-scans the tree and persists its verdict to
  `.argot/health.json`; **`argot check` reads it and prints one-line notes**
  when new generated/data-heavy directories are shaping the voice or when
  `argot.toml` changed since the fit (in `--format github`, these become
  run-level PR annotations),
- **editing `[exclude]`/`[detect]` is itself a refresh trigger**: the next
  check notices the config fingerprint changed and refits in the background,
- **`argot status`** is the one-stop health view: fitted SHA, commits behind,
  config in sync or not, and unexcluded noisy directories,
- a failing background refit stops retrying silently and tells you to run
  `argot fit` yourself.

A well-configured repo stays quiet on all of it.

## `[update]` — the passive update notice

argot prints at most one dim line a day on stderr when a newer release exists
(`argot X is available — run 'argot update'`). It's a single cached GET to
`https://argot.tmonier.com/version.json` at most **once per 24 hours**,
refreshed in a detached process so it never adds latency, and it's
automatically silent in CI, on a non-tty, under `--quiet`, in machine formats,
and when `ARGOT_OFFLINE` is set. To opt out entirely:

```toml
[update]
check = false          # or set ARGOT_UPDATE_CHECK=0 in the environment
```

This version check and the one-time embedding-model download are the **only**
network calls argot ever makes on its own — nothing else ever leaves your
machine.

## Environment variables

| Variable | Effect |
|---|---|
| `ARGOT_SEMANTIC_MODEL=<path>` | Use a local GGUF embedding model — skips the download entirely (air-gapped installs, CI with a pre-fetched model). |
| `ARGOT_OFFLINE=1` | Never touch the network. If the model isn't cached, the semantic rules are skipped with a printed note — never silently. |
| `ARGOT_MODEL_URL=<url>` | Fetch the embedding model from a mirror (corporate artifact store). The sha256 is verified regardless of source. |
| `ARGOT_UPDATE_CHECK=0` | Disable the passive update notice (same as `[update] check = false`). |

The model download also honors the standard `HTTPS_PROXY` / `HTTP_PROXY` /
`ALL_PROXY` variables.

## Which files argot writes, and where

Only `argot.toml` (at the repo root) is meant to be committed. Everything under
`.argot/` is a rebuildable artifact that `argot fit` regenerates in seconds, so
`init`/`fit` drop a `.argot/.gitignore` that keeps the whole directory out of
version control.

| File | Written by | What it is | Committed? |
|---|---|---|---|
| `argot.toml` *(repo root)* | `init` / `argot mute` / by hand | Config — `[exclude]`, `[detect]`, `[rules]`, `[update]`, and `[[mute]]`. | **Yes** — commit it. |
| `argot.local.toml` *(repo root)* | you | Personal overrides, merged on top. | No — gitignored. |
| `.argot/scorer-config.json` | `fit` / `init` | The fitted voice model: calibrated threshold(s) + scorer config. | No — rebuildable. |
| `.argot/semantic-index.json` | `fit` / `init` | The per-repo code-embedding index for the reinvention/placement checks. Records the model that built it — an index from a different model/version is rejected with a "run `argot fit` to rebuild" message rather than scoring wrong. | No — rebuildable. |
| `.argot/layering.json` | `fit` / `init` | The module-dependency graph the `layering` rule checks added imports against. | No — rebuildable. |
| `.argot/manifest.json` | `fit` / `init` | Versioned, hashed record of what was learned (model hash, fit commit, corpus size); read by `inspect --model`. | No — rebuildable. |
| `.argot/repo-corpus.txt` | `fit` / `init` | The source files counted into the repo distribution. | No — rebuildable. |
| `.argot/generic-baseline.json` | `fit` / `init` | The bundled generic-baseline reference. | No — rebuildable. |
| `.argot/dataset.jsonl` | `extract` | Raw training dataset — one record per hunk. The check path doesn't need it. | No — rebuildable. |
| `.argot/last-check.json` | `check` | Cache of the last check's hits, so `argot mute <hash>` can resolve. | No — rebuildable. |
| `.argot/.gitignore` | `fit` / `init` | Ignores everything under `.argot/`. | Ignores itself; regenerated by `fit`/`init`. |

Want to commit the model yourself instead? Delete `.argot/.gitignore` and it
stays out of your way — CI otherwise restores the model from cache or re-fits.

> **The embedding model lives outside the repo.** The semantic layer's
> code-embedding model (~100 MB) is fetched once to a shared user cache —
> `~/.cache/argot/models/` on macOS **and** Linux (`XDG_CACHE_HOME` respected),
> `%LOCALAPPDATA%\argot\models\` on Windows — not into `.argot/`, so it's shared
> across every repo and never committed. The sha256 is verified at download *and*
> on every cache hit, a `CACHEDIR.TAG` marks the directory for backup tools, and
> superseded model files are garbage-collected after a new one lands. Manage it
> explicitly with `argot model fetch` / `status` / `clean` — see
> [The commands](/docs/the-commands/#model). The `.argot/semantic-index.json` it
> produces is gitignored with the rest of `.argot/`.

## Which to reach for

| You want to… | Use |
|---|---|
| Stop argot learning from a directory | `[exclude].paths` in `argot.toml` |
| Learn from one recommended dir | remove its entry from `[exclude].recommended` |
| Turn the recommended set off entirely | `recommended = []` in `[exclude]` |
| Catch a repo's own codegen banner | add it to `[detect].generated-markers` |
| Tune how aggressively data files are dropped | `[detect].data-threshold` |
| Make a rule report-only, or turn it off | `[rules]` in `argot.toml` (or `--rule` per run) |
| Accept one deliberate line | inline `# argot: ignore-next-line — reason` |
| Accept a specific reported hit for good | `argot mute <hash> --reason …` |
| Temporarily silence a hit | `argot mute <hash> --expires 30d` |
| Cover many hits under one path | a `path:` glob `[[mute]]` rule |
| Exclude something on your machine only | `argot.local.toml` |

Suppressed ≠ deleted: `check` drops muted hits from its output and exit code but
still prints a one-line count on stderr, so a repo full of silent mutes never
looks cleaner than it is.
