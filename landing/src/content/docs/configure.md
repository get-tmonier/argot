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

`[exclude]` has **three** gitignore-style pattern lists that differ only in how a
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

# Paths that are CHECKED like any other, but never shape the voice. argot learns
# their dependency vocabulary, not their style. Defaults to your tests.
check-only = [
  "test/", "tests/", "__tests__/", "benchmarks/",
  "test_*", "*.test.*", "*.spec.*",
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

One scope decision argot makes for you: **gitignored files never shape the
voice.** The fit reads files from disk, but anything `.gitignore` covers and
git doesn't track — editor-history trees like `.history/`, local worktrees,
build output — is dropped from the training corpus automatically. Only
*committed* code needs an `[exclude]` entry.

Don't hand-guess the directories to exclude — [`argot init --suggest`](/docs/setup/)
surfaces them with evidence, on two grounds: directories whose files are
generated or data-heavy, and directories your repository **stores but never
writes**. The second is the one that hides in plain sight — a vendored library,
a forked upstream copy, a machine-translated binding. That code carries no
marker and reads like anything else, but its history gives it away: it arrived
in one commit and the repo has barely touched it since, while the code around it
is edited constantly. argot reports the ratio and lets you decide. See
[Setup](/docs/setup/).

### `check-only` — checked, but never teaches

The third list answers a question the other two can't: *judge this code, but
don't learn from it.*

Tests are the reason it exists. A test file's phrasing is deliberately unlike
production code — arrange/act/assert, fixtures, mocks — so learning style from
tests dilutes the voice. But a test's **dependencies** are real vocabulary: a
harness or a clock-control library that only ever appears in tests is not a
foreign dependency, it's how this repo tests.

A path in `check-only` is:

- **out of the voice corpus** — no surprisal, no callee clusters, no
  thresholds. Test style never mixes with production conventions.
- **in the vocabulary** — the import specifiers those files use are learned into
  a separate set, consulted *only* when scoring one of those same files. A
  dependency only your tests use stays foreign in production code.
- **checked** — but by the import signal alone. On these paths the voice reports
  only `foreign-import`; `rare-tokens`, `unfamiliar-callee` and `convention`
  are withheld, because the model never read a file like this one. Every other
  rule — custom rules, `layering`, the test-integrity rules — behaves normally.

**By default this list changes nothing**, because the same paths are also in
`recommended`, which drops them from checking entirely. To turn your tests into
a guarded, non-teaching scope, remove the test patterns from `recommended` and
leave them here:

```toml
[exclude]
recommended = [
  "docs/", "build/", "dist/",     # … the rest of the defaults, minus:
  # "test*/", "__tests__/", "test_*", "*.test.*", "*.spec.*",
]
check-only = ["test*/", "__tests__/", "test_*", "*.test.*", "*.spec.*"]
```

Re-run `argot fit` afterwards — the test vocabulary is learned at fit time. It
is a membership set, not a distribution, so it needs no minimum corpus: three
test files are enough for the three imports they name, and a repo with no tests
gets an empty set and today's behaviour exactly.

A bare pattern here names a **file** (`test_*` matches `test_helpers.py`, not a
`test_util/` directory of production support code); add a trailing slash for a
directory (`tests/`), or use a path shape (`**/__tests__/**`).

### Editing the `recommended` set

`init` writes the full built-in `argot:recommended` set into `recommended` — the
directories and files that are almost never part of a repo's authored voice:

- **Directories** (matched at any depth): `test*/` (any dir starting `test`),
  `__tests__/`, `doc/` `docs/` `example/` `examples/` `migration/` `migrations/`
  `benchmark/` `benchmarks/` `fixtures/` `scripts/` `build/` `dist/`
  `__pycache__/` `.git/` `.history/` `.tox/` `.eggs/`.
- **Files:** `test_*` and `conftest.py`; `*.test.*` / `*.spec.*` / `*.config.*`
  (so `x.test.ts`, `vite.config.ts`); and dotfile `.*rc.*` configs (`.babelrc.js`).

Because it's a plain list, you tune it **per entry**: remove `"test*/"` to bring
tests back into scope (`check-only` above still keeps them out of the voice), add
`"vendor/"` to drop a vendored tree silently, or set
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
belongs to one of **twelve stable rules**, in five groups:

| Group | Rules |
|---|---|
| `voice` | `foreign-import` · `unfamiliar-callee` · `rare-tokens` · `convention` · `superseded` |
| `semantic` | `redundant` · `misplaced` |
| `architecture` | `layering` |
| `integrity` | `test-deleted` · `test-disabled` · `test-weakened` |
| `governance` | `rule-tampered` — the guardrail's self-protection (see [Locked rules](#locked-rules--the-agent-cant-turn-off-the-alarm)) |

A repo can add a **sixth group, `custom`**: repo-local rules dropped under `.argot/rules/`,
discovered fresh on every run and configured exactly like the built-ins — see
[Custom rules](/docs/custom-rules/).

Each rule carries a **severity**: `error` (reported, fails `argot check` with
exit 1), `warn` (reported, does not fail the check), or `off` (the rule does not
run). **Everything defaults to `error`** (except `test-weakened`, which ships
`warn` — reported, never failing the check) — argot gates on the rest until you
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

The optional Claude Code pre-write hook has no separate configuration format. It
reads this same effective `[rules]` resolution: a foreign-import rule set to
`off` does not ask, while `warn` and `error` retain the hook's non-blocking
prompt. Configure the hook itself through the plugin/setup flow; configure its
rule policy here.

Two companions on the CLI:

- **`argot rules`** (`--format json` for machines) lists every rule with its
  group, effective severity for this repo, and a one-line description.
- **`argot check --error-on-warnings`** makes `warn`-severity findings fail the
  check too — strict-CI mode without touching the committed config.

Severity is about the **exit code**. It's distinct from the *confidence* tier
(`unusual` / `suspicious` / `foreign`) a hit displays with, which grades the
evidence — see [Reading the output](/docs/reading-the-output/).

### Path-scoping a rule

Any rule — built-in or custom — can be limited to (or kept out of) a set of paths, from the
`[rules]` inline-table form:

```toml
[rules]
layering   = { severity = "error", include = ["src/**"] }   # only enforce layering under src/
convention = { exclude = ["legacy/**", "vendor/**"] }        # everywhere except these trees
custom     = { include = ["packages/api/**"] }               # scope the whole custom group
```

`include` keeps only findings whose file matches; `exclude` drops findings whose file matches
(and wins over `include`). Both use the same glob dialect as `[[mute]].path` (`*`/`**` cross
`/`). A rule-specific entry beats its group's. This is a filter on **findings** — the rule still
runs, its out-of-scope hits are simply dropped — so it composes with everything else here.

This is distinct from a *custom rule's* manifest `include`/`exclude` (see
[Custom rules → which files a rule runs on](/docs/custom-rules/#which-files-a-rule-runs-on)):
the manifest decides which files the rule ever *runs* on (and is the only way to reach files
argot doesn't score, like `.env`); the `[rules]` scope here is the repo owner narrowing any
rule's findings after the fact. `rule-tampered` is never path-scoped away.

### Locked rules — the agent can't turn off the alarm

Opt-in strict mode, per rule or per group, from the **committed** `argot.toml` only:

```toml
[rules]
layering = { severity = "error", locked = true }
custom   = { severity = "error", locked = true }   # lock every repo-local rule
```

A locked rule is frozen against every runtime relaxation an AI agent might reach for
when a check fails:

- **Severity is pinned** at the committed value — `argot.local.toml` and `--rule`
  overrides are refused, with a warning on stderr.
- **Every suppression surface is refused for its findings** — inline
  `# argot: ignore…` comments, `[[mute]]` entries, and `[exclude].paths` do not
  apply. The lock means locked.
- **Weakening the lock is itself a finding.** The `rule-tampered` rule (group
  `governance`) reads both sides of the change being checked: a diff that removes a
  lock, downgrades a locked rule's severity, adds a `[[mute]]` targeting a locked
  rule, or edits a locked custom rule's script/manifest fires an **error** with the
  exact weakening named — and a run-level warning that CI surfaces loudly
  (`--format github` turns it into a PR annotation).
- `rule-tampered` itself is **pinned**: always `error`, always locked, never
  suppressable. An alarm you can configure off is not an alarm.

This is tamper-*evidence*, not tamper-proofing — the same philosophy as the
`integrity` rules: an agent *can* touch the alarm, but touching the alarm **is**
the alarm. The one quiet path to relaxing a locked rule is a committed
`argot.toml` diff that a human reviews.

## Inline comments — mute one line

> **Locked rules ignore inline ignores.** A `# argot: ignore` comment has no effect on a rule
> locked with `locked = true` — see [Locked rules](#locked-rules--the-agent-cant-turn-off-the-alarm).

**Location:** in the source file itself, on the line above the code (or around
the block) you're excusing. The comment token is the language's own line comment:
**`#` for Python and Ruby, `//` for everything else** (TypeScript/JavaScript, Go,
Rust, C, C++, Java, C#, PHP, Pascal) — the language adapter supplies it.

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
  `layering`, `test-deleted`, `test-disabled`, `test-weakened`) or a whole group
  (`voice`, `semantic`, `architecture`, `integrity`). Omit it to mute the hunk
  whatever fired. An unknown rule name is a warning, and the directive is
  ignored.
- **`ignore-next-line`** mutes the single line below the comment.
  **`ignore-block-start` … `ignore-block-end`** mute everything between them
  (the `-end` needs no reason). An unclosed block suppresses to end of file with
  a warning.

## `[[mute]]` — accept a specific hit

> **A `[[mute]]` can't silence a locked rule** — the entry is refused, and *adding* one that
> targets a locked rule is reported by `rule-tampered`. See
> [Locked rules](#locked-rules--the-agent-cant-turn-off-the-alarm).

**Location:** the `[[mute]]` tables in `argot.toml`. `argot mute` appends one;
you can also hand-edit it. It's **committed**, so a mute is a shared, reviewable
audit trail that a teammate and CI inherit.

There are two forms, and picking the wrong one is how a repo ends up with a
committed mute per file.

**Per hit — by hash.** Every hit prints a stable `[hash]`:

```text
argot mute a1b2c3d4e5f6 --reason "adopting axios repo-wide"
argot mute a1b2c3d4e5f6 --reason "temporary shim" --expires 30d
```

A hash pins **that hit and no other**. The identical finding in a sibling file
has its own hash and stays flagged — which is what you want for a genuine
one-off, and not what you want for a standing decision.

**Standing — by path.** When the decision covers a tree, name the tree:

```text
argot mute --path 'src/legacy/**' --rule foreign-import --reason "migrating in Q3"
argot mute --path 'vendor/**' --reason "vendored upstream" --expires 90d
```

`--path` takes the same globs as `[exclude].paths`; `--rule` narrows it to one
rule or group (validated against your repo's full vocabulary, custom rules
included, so a typo is refused rather than silently ignored). It covers every
future hit under the glob and needs no prior `check` run.

`--reason` records why (recommended everywhere argot reports a hit); `--expires`
takes a **day count** (`30d`, or a bare `30`), which argot resolves to a calendar
date in the file. The hash form reads the last `check` run to learn which file
the hash belongs to, so run `argot check` first. Either append is a
format-preserving edit, so your hand-written sections and comments are never
rewritten.

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

# path + reason alone is the standing form — it covers every hit under the glob.
# `argot mute --path` writes exactly this; hand-editing does the same thing:
[[mute]]
path = "generated/**"
reason = "protobuf stubs, never our voice"
```

Only `path` and `reason` are mandatory. An entry with a missing/invalid field (no
`path`, no `reason`, an unknown `rule`, a malformed `expires`) is skipped with a
warning on stderr — one bad entry never voids the file. An entry with `expires`
in the past is treated as expired and ignored; one expiring **today** is still
active.

## `[[migration]]` — declare a migration

*This is the config surface for the `superseded` rule's **declared** side. argot also
**mines** the same fact from your accepted history — no config needed — and the rule
itself (evidence, severity, the leftover report) is documented end to end in
[What it catches](/docs/what-it-catches/#superseded--new-code-written-the-old-way).*

A repo mid-migration has two voices, and a model trained on history alone only hears
the loud (old) one until enough commits accumulate. `[[migration]]` states the
replacement yourself, and it's enforced from the very next `check` — no refit needed:

```toml
[[migration]]
from = "moment"
to = "date-fns"
reason = "Q2 date-handling refactor"
# kind = "callee"   # optional: "import" (default) or "callee"
```

- **`from`, `to`, and `reason` are mandatory** — the same discipline as `[[mute]]`. An
  entry missing one, or naming an unknown `kind`, is skipped with a warning on stderr;
  one bad entry never voids the file.
- **`kind`** is optional: `"import"` (the default) for a module specifier, or
  `"callee"` for a method/function call.
- **`argot.local.toml` entries append** to the committed ones, exactly like `[[mute]]`.
- **Effective immediately, no refit needed.** A declared migration is read at check
  time, not baked into the fitted model: `to` stops reading as foreign and `from`
  starts raising `superseded` from the very next `check`. Declaring one doesn't even
  trigger the background auto-refresh — `[[migration]]` isn't part of the config
  fingerprint `[fit]` freshness watches (see [`[fit]`](#fit--the-background-auto-refresh)).
- **Scoping reuses the existing per-rule path scopes** — no separate mechanism:
  `[rules] superseded = { severity = "warn", include = ["src/**"] }` limits where the
  rule fires (mined or declared alike), exactly like
  [scoping any other rule](#path-scoping-a-rule).
- The finding it produces is a rule like any other: `[rules] superseded = "off"` turns
  it off (per repo or per run), `[[mute]]` and inline
  `# argot: ignore-next-line rule=superseded — reason` suppress one hit, and
  `argot rules` lists its effective severity (`warn` by default — reported, never
  failing the check on its own).

## `argot.local.toml` — personal overrides

Want a scratch directory excluded on *your* machine only, without touching the
committed config? Put it in **`argot.local.toml`** at the repo root — `init`
gitignores it for you. It deep-merges over `argot.toml`: scalars there win,
list entries (`paths`, `generated-markers`, `[[mute]]`, `[[migration]]`) append, and
`[rules]` entries override the committed ones key by key (the CLI's `--rule` still
beats both).

```toml
# argot.local.toml — gitignored, personal, uncommitted.
[exclude]
paths = ["scratch/", "experiments/"]   # appended to the committed excludes
```

## `[fit]` — the background auto-refresh

*This is the config surface; the full mechanism — verdict, health record,
drift, staleness — is one page:
[Health & freshness](/docs/health-and-freshness/).*

A fit is a snapshot: as the repo merges new dependencies and modules, a stale
model starts reading your own accepted code as foreign. argot keeps itself
fresh — when **accepted history** gains `refresh-after` commits touching
in-scope source since the fit (or the fit is more than a week old with any
such drift), a `check` refits **in the background**, detached, at most once a
day, and says so in one dim line. The check you just ran used the old model
(zero added latency); the next one scores against the fresh voice.

"Accepted" is the load-bearing word: staleness is measured — and the refit
fitted — at the **merge-base with your default branch**, in a throwaway
worktree when needed. A feature branch's own commits and your uncommitted
edits never train the voice; they stay the code under judgment. Only commits
touching in-scope source count, so docs and CI churn don't age the voice
either. The semantic index reuses the embeddings of unchanged functions, so a
routine refresh costs seconds. CI never auto-refits (the Action refits per
base advance). `init` writes the defaults explicitly:

```toml
[fit]
auto-refresh = true               # false: you drive `argot fit` yourself
refresh-after = 10                # accepted in-scope commits before a refresh
refresh-from = "default-branch"   # auto-detected — see below
```

`refresh-from` is a mode, not a blank to fill in: `"default-branch"`
auto-detects your trunk (`origin/HEAD`, else `main`, else `master`), so main-
and master-only repos both just work. Name a branch (`"develop"`) when your
trunk is non-standard, or set `"current-branch"` to let refreshes learn
whatever HEAD has.

Freshness is separate from **calibration drift** — a new `gen/` dir or a
vendored SDK appearing in the tree, or an `argot.toml` edit the model hasn't
absorbed yet. argot watches for both itself, and you never have to guess when
to recalibrate:

- every `fit`/`init` re-scans the tree and persists its verdict to
  `.argot/health.json`; **`argot check` reads it and prints one-line notes**
  when new generated, data-heavy, or vendored directories are shaping the
  voice or when
  `argot.toml` changed since the fit (in `--format github`, these become
  run-level PR annotations),
- **editing `[exclude]`/`[detect]` is itself a refresh trigger**: the next
  check notices the config fingerprint changed and refits in the background,
- **`argot status`** (`--repo <path>` for a repository you are not sitting in)
  is the one-stop health view: fitted SHA, commits behind,
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
| `.argot/scorer-config.json` | `fit` / `init` | The fitted voice model: calibrated threshold(s) + scorer config. Also records, per language, how the threshold scales with hunk size (`size_slope`, `size_reference_lines`) — a hunk's score is a max over its tokens, so a large one scores higher for free, and the bar rises to match above the repository's own p90. Fitted from your candidates; nothing to set. | No — rebuildable. |
| `.argot/semantic-index.json` | `fit` / `init` | The per-repo code-embedding index for the reinvention/placement checks. Records the model that built it — an index from a different model/version is rejected with a "run `argot fit` to rebuild" message rather than scoring wrong. | No — rebuildable. |
| `.argot/layering.json` | `fit` / `init` | The module-dependency graph the `layering` rule checks added imports against. | No — rebuildable. |
| `.argot/integrity.json` | `fit` / `init` | Per-repo learned gates for the test-integrity rules (`test-deleted` / `test-disabled` / `test-weakened`), from a mini-replay of the repo's accepted history. | No — rebuildable. |
| `.argot/manifest.json` | `fit` / `init` | Versioned, hashed record of what was learned (model hash, fit commit, corpus size); read by `inspect --model`. Names any language too thin to learn a voice from (`unlearnable_languages`) — those files are **not checked**, and the skip is recorded rather than silent. | No — rebuildable. |
| `.argot/health.json` | `fit` / `init` | The fit's self-record — fitted SHA, config fingerprint, drift candidates; read by `check` and `status` for the freshness notes ([Health & freshness](/docs/health-and-freshness/)). | No — rebuildable. |
| `.argot/repo-corpus.txt` | `fit` / `init` | The source files counted into the repo distribution. | No — rebuildable. |
| `.argot/generic-baseline.json` | `fit` / `init` | The bundled generic-baseline reference. | No — rebuildable. |
| `.argot/dataset.jsonl` | `extract` | Raw training dataset — one record per hunk. The check path doesn't need it. | No — rebuildable. |
| `.argot/last-check.json` | `check` | Cache of the last check's hits, so `argot mute <hash>` can resolve. | No — rebuildable. |
| `.argot/auto-refit.json` + `.lock` | background refresh | Attempt/result state and the one-refit-at-a-time lock ([Health & freshness](/docs/health-and-freshness/)). | No — state. |
| `.argot/.gitignore` | `fit` / `init` | Ignores everything under `.argot/`. | Ignores itself; regenerated by `fit`/`init`. |
| `.argot/rules/<name>/` | `argot-write-rule` / `argot-suggest-rules` / by hand | Your [custom rules](/docs/custom-rules/) — `rule.toml` + `check.rhai`, authored not generated. | **Yes** — force-add past `.argot/.gitignore`; `uninstall` never deletes them. |

Outside the repo, argot keeps exactly two user-level locations:

| Location | What it is |
|---|---|
| `~/.argot/settings.json` | The global repo registry — every repo argot has fitted or checked, powering `argot list`/`status` (and telling `argot uninstall` where artifacts live). |
| `~/.cache/argot/` | The embedding-model cache and the once-a-day update-check state (`update-check.json`) — see the callout below. Curl installs also leave a receipt in `~/.config/argot/`. |

To remove all of it — every repo's artifacts, the cache, the registry, and the
binary — run [`argot uninstall`](/docs/the-commands/#uninstall): it shows the
full inventory first and never touches git-tracked files or your authored custom
rules under `.argot/rules/`.

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
| Limit a rule to (or out of) some paths | `include`/`exclude` on the `[rules]` entry |
| Accept one deliberate line * | inline `# argot: ignore-next-line — reason` |
| Accept a specific reported hit for good * | `argot mute <hash> --reason …` |
| Temporarily silence a hit * | `argot mute <hash> --expires 30d` |
| Cover many hits under one path * | a `path:` glob `[[mute]]` rule |
| Make a rule un-silenceable | `locked = true` on its `[rules]` entry |
| Exclude something on your machine only | `argot.local.toml` |

**\*** The suppression surfaces (inline ignore, `[[mute]]`, `[exclude].paths`) do **not** apply
to a [**locked** rule](#locked-rules--the-agent-cant-turn-off-the-alarm) — that's the point of a
lock, so an AI agent can't quiet a rule it can't satisfy. Weakening a lock is itself reported by
`rule-tampered`.

Suppressed ≠ deleted: `check` drops muted hits from its output and exit code but
still prints a one-line count on stderr, so a repo full of silent mutes never
looks cleaner than it is.
