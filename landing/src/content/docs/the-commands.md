---
title: The commands
description: init and check — the everyday commands — plus fit, rules, model, and the on-demand tools.
group: Guide
order: 4
---

The two everyday commands are **`argot init`** (one-time setup — it fits the model and health-checks
the repo) and **`argot check`** (the per-diff loop). `fit` is what `init` runs under the hood. The
rest — `rules`, `model`, `review`, `voice-diff`, `inspect`, `mute` — are on demand. Run
`argot --help` for the full list.

## init

Fits the voice model to the repo (`fit`), prints a health check (corpus composition + a
Ready / Marginal / Not-recommended verdict — [what drives it](/docs/health-and-freshness/#the-verdict--ready--marginal--not-recommended)),
and writes a `.argot/.gitignore` so the rebuildable
model stays out of version control. This is the one command a new repo needs.

```bash
argot init                   # set up the current repo
argot init --suggest         # list generated/data-heavy dirs you may want to exclude first
argot init --suggest --format json   # the same, machine-readable (for the setup skill)
```

See [Setup](/docs/setup/) for deciding what shouldn't shape your voice.

## fit

One-shot voice fitting: collects the repo's source files as the repo corpus, sets up the generic
baseline, then samples representative hunks to set the scoring threshold.

```bash
argot fit
```

Fitting on a feature branch whose unmerged commits touch in-scope source earns a
warning (never a prompt): those commits become the voice, and argot stops flagging
what it has learned. The background auto-refresh never does this — see
[Health & freshness](/docs/health-and-freshness/#staying-fresh--the-background-auto-refresh).

Writes its artifacts under `.argot/`:

| File | What it is |
|---|---|
| `repo-corpus.txt` | the source files counted into the repo distribution |
| `generic-baseline.json` | the bundled generic baseline reference |
| `scorer-config.json` | the calibrated threshold(s) and scorer config |
| `semantic-index.json` | the per-repo code-embedding index for the reinvention/placement checks |
| `layering.json` | the module-dependency graph the `layering` rule checks added imports against |

`fit` also builds the **semantic index**: it embeds every function with a local code-embedding model
(`jina-code`, ~100 MB, fetched once to a local cache on first use — pre-fetch it with
[`argot model fetch`](#model)) and writes `.argot/semantic-index.json`. This is standard — there is
no flag to enable it, though setting the `semantic` rule group to `off` in `argot.toml`'s `[rules]`
skips it (no download, no index). It also refreshes
`.argot/manifest.json` (the hashed model record). For every file argot writes,
where it lives, and whether it's committed, see the
[reference table in Configure](/docs/configure/#which-files-argot-writes-and-where).
Re-run `fit` after a major refactor. Internally it runs the engine's two underlying phases (build
corpus, then calibrate); both stay available as engine entry points for benchmark and research use.

### Sliced calibration (per-subdirectory / per-author)

One repo can hold more than one voice — a `frontend/` and a `backend/`, or different contributors.
`--slice` calibrates an extra threshold for a slice of the repo; at check time a hunk is judged
against its slice's threshold instead of the whole-repo one.

```bash
argot fit --slice auto                     # one threshold per top-level directory
argot fit --slice path:frontend/           # an explicit subdirectory
argot fit --slice author:alice@example.com # the files an author owns
```

Slices are additive: the whole-repo threshold still applies to anything outside a slice, and a slice
with too few calibration candidates is skipped (it would only be noisier). Multiple `--slice` flags
combine; the first matching slice wins.

> **Privacy (per-author).** An `author:` slice is derived from your local git history and stored as a
> list of that author's files inside `.argot/scorer-config.json`. If you commit `.argot/`, those file
> lists (and the author email in the slice name) travel with it — gitignore `.argot/` if that's
> sensitive. argot never sends anything off the machine.

## check

Scores changed hunks against the trained scorer and prints them grouped by file. Alongside the base
voice model, `check` also runs the semantic layer's **reinvention** (`redundant`) and **placement**
(`misplaced`) checks against `.argot/semantic-index.json`, and the **architecture** check
(`layering`) against `.argot/layering.json` — automatically, no flag. (The first check that embeds
may pause briefly to fetch the ~100 MB model to a local cache; after that it's warm. Pre-fetch with
[`argot model fetch`](#model).)

**Exit codes:** `0` clean · `1` at least one `error`-severity finding — *something to look at, not
a verdict* · `2` setup/usage error. What exits 1 is the rule's configured **severity**: every rule
defaults to `error` (except `test-weakened`, which ships `warn`), and any rule set to `warn` is still reported but doesn't fail the check
(`--error-on-warnings` flips that back on for strict CI). Confidence tiers
(`unusual`/`suspicious`/`foreign`) grade the evidence for display — they never drive the exit code.
See [Configure](/docs/configure/#rules--rule-severities).

```bash
argot check                         # uncommitted changes — modified + staged + untracked
argot check --staged                # staged changes only
argot check --unstaged              # unstaged changes only
argot check HEAD~5                  # everything from HEAD~5 to current state
argot check HEAD~5..HEAD            # commits in that range only
argot check --commit abc1234        # a single commit
```

### Scoping and filtering

```bash
argot check --only 'src/*'            # restrict to matching files (repeatable)
argot check --exclude 'test/*'        # drop matching files (repeatable; wins over --only)
argot check --min-confidence foreign  # only show foreign-confidence hits
argot check --rule misplaced=warn     # override a rule's severity for this run (repeatable)
argot check --rule semantic=off       # …or a whole group
argot check --error-on-warnings       # warn-severity findings also fail the check (strict CI)
argot check --verbose                 # show full hunk contents (no truncation)
```

`--min-confidence` filters the *display* by evidence tier. Keep the default (`unusual`) to see
everything argot flags — a lone foreign import can score right at the threshold and land in
`unusual`, so `--min-confidence foreign` (the strongest-evidence tier) may *hide* a single new
dependency. Raise it to `suspicious` or `foreign` only to cut noise on a chatty repo, once you
trust the calibration.

`--rule <name|group>=<error|warn|off>` overrides the committed `[rules]` config for one run — CLI
beats `argot.local.toml` beats `argot.toml` beats the all-`error` defaults. `argot rules` lists
what's in effect.

Every `check` run also names the model that judged the diff — a short `model:` hash on stderr (human)
or in the `model` field of `--format json`/`sarif`. Same corpus + config always fits the same hash, so
you can tell at a glance whether your model matches a colleague's.

### Output and advanced flags

```bash
argot check --format json           # stable machine JSON (human | json | sarif | github; default human)
argot check --format sarif          # SARIF 2.1.0 for code-scanning uploads
argot check --format github         # GitHub Actions workflow commands → inline PR annotations
argot check --quiet                 # suppress informational stderr (model line, nudges, counts)
argot check --hunk-lines 12         # lines of hunk body under each hit (default 6; 0 to suppress)
argot check --repo ../other-repo    # check a repo other than the current directory (default .)
```

| Flag | Default | What it does |
|---|---|---|
| `--format` | `human` | `human`, `json` (stable schema), `sarif` (SARIF 2.1.0), or `github` (Actions workflow commands — inline PR annotations with no upload step). Machine formats write only the document to stdout — see [Reading the output](/docs/reading-the-output/). |
| `--rule <name>=<sev>` | — | Override a rule or group's severity (`error`/`warn`/`off`) for this run. Repeatable. |
| `--min-confidence <tier>` | `unusual` | Only show hits at or above this confidence tier. |
| `--error-on-warnings` | off | Exit non-zero when `warn`-severity findings are present. |
| `--quiet` / `-q` | off | Suppress informational stderr notes. Errors still print. |
| `--add-ignores` | off | Instead of reporting, insert an inline `# argot: ignore-next-line rule=… — baselined by --add-ignores; review` comment above every current finding — the adoption move on an existing codebase (working-tree modes only). Review the comments, then commit them. |
| `--repo <path>` | `.` | Repository to check. |
| `--argot-dir <path>` | `.argot` | Where to load the fitted model from. A relative path is resolved against `--repo`; an absolute path is used verbatim. |
| `--hunk-lines <N>` | `6` | Hunk-body lines shown under each hit (`0` suppresses them; `--verbose` overrides with the full hunk). |

Color follows the [`NO_COLOR`](https://no-color.org) convention: argot colors confidence markers only when
`NO_COLOR` is unset **and** stdout is a terminal. Machine formats are never colored.

### Freshness

A fit that falls **`refresh-after` accepted source commits behind** (default
10 — measured on your default-branch line, so a feature branch's own commits
and docs churn never count; or a week old with any such drift) is refreshed
automatically: `check` spawns a detached background refit (at most once a day,
never in CI, fitted at the accepted anchor so unmerged work never trains the
voice) and tells you in one dim line — the next check uses the fresh voice.
Opt out with `[fit] auto-refresh = false`. The whole self-maintenance loop —
verdict, health record, drift, staleness — is one page:
[Health & freshness](/docs/health-and-freshness/).

## rules

List every rule with its group and the **effective severity** for this repo — the resolved result
of the defaults, `argot.toml`, and `argot.local.toml`:

```bash
argot rules                  # RULE / GROUP / SEVERITY / DESCRIPTION table
argot rules --format json    # the same, machine-readable
```

Ten rules in four groups: `voice` (`foreign-import`, `unfamiliar-callee`, `rare-tokens`,
`convention`), `semantic` (`redundant`, `misplaced`), `architecture` (`layering`), and `integrity`
(`test-deleted`, `test-disabled`, `test-weakened`). Configure them in `argot.toml`'s `[rules]` or
per run with `check --rule` — see [Configure](/docs/configure/#rules--rule-severities).

## model

Explicit control over the semantic layer's fetched-on-first-use embedding model (~100 MB GGUF). The
automatic path needs none of this — these exist for CI pre-warming, air-gapped installs, and cache
hygiene:

```bash
argot model fetch            # download and verify the model now (instead of on first use)
argot model status           # is it present, where, and how big
argot model clean            # delete the model cache (re-fetched on next use)
```

`fetch` fails loudly (exit 2 with the reason) instead of degrading — that's the point: run it in CI
setup or before going offline, and a network problem surfaces there rather than as a skipped check
later. Downloads verify the sha256, honor `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY`, and respect
`ARGOT_MODEL_URL` (mirror), `ARGOT_SEMANTIC_MODEL` (local file, no download), and `ARGOT_OFFLINE`
— see [Configure](/docs/configure/#environment-variables).

## audit

```bash
argot audit                    # what did AI sneak into your last 50 commits?
argot audit --commits 200      # wider window
argot audit --since 6m         # by time instead: a duration (90d, 12w, 6m, 1y)…
argot audit --since 2026-01-01 # …or a date
argot audit --format json      # stable schema; also: markdown (PR-pasteable),
                               # html (self-contained, screenshot-ready)
```

The install-day question, answered on your own history: audit fits the voice
**as it was at the base of the window** (in a temporary git worktree — your
tree and `.argot/` are untouched; your current `argot.toml` and semantic index
ride along, so the historical fit reuses embeddings), scores `base..HEAD` with
every rule group, and renders a scorecard. It works on a fresh clone with no
setup at all — no `.argot/`, no `argot.toml`. Most repos audit in seconds to a
couple of minutes; on a very large repo the first zero-setup run builds the
full semantic index (minutes, with live progress) — after `argot init` the
index seeds the fit and repeat audits are several times faster.

Every finding is attributed to its **introducing commit** — `ai-assisted`,
`human`, or `unknown` — from concrete commit markers only: agent
`Co-authored-by` trailers (Claude Code, Copilot, Cursor, Codex, aider, …),
agent bot authors, and agent footer lines. argot never guesses from style, so
the headline AI share is a floor, not a census, and the card says so.

The card leads with one headline (window, commits, AI share, findings), then
per-group counts in plain language, then the single worst offender as a
concrete story — commit, file, evidence line — with the rest below the fold.
The framing stays honest: merged code is accepted code, so each finding reads
as *"would have prompted review before merge"* — a fire on a dependency you
adopted deliberately is a detection working as intended. If your in-scope code
is younger than the window (a rewrite, or early history your current excludes
mute entirely), audit shrinks the window to the oldest commit it can fit and
says so; oversized windows clamp loudly (cap: 1,000 commits), never silently.
Informational: always exits 0 (2 when it can't run).

## review

Score a pull request against your local voice **without checking it out** — the
moment argot is most useful is at the merge button, reviewing someone else's PR.
`review` fetches the PR head into your object store (a fetch, not a checkout —
your working tree is never touched), then runs the same scoring and output as
`check` with a PR header on top.

```bash
argot review 123                                  # PR #123 in the current repo
argot review https://github.com/org/repo/pull/45  # by URL
argot review 45 --repo org/repo                    # PR in another repo
argot review main..HEAD                            # any diff range (no gh needed)
argot review abc1234                               # a single commit
```

PR mode uses the `gh` CLI (`gh auth login` once). Range and commit targets go
straight through to the local git — no network. `--format json|sarif|github`
works the same as `check`. `review` also prints a one-line **voice-diff**
headline above the hits.

## voice-diff

A single PR-level number — *how out of voice is this diff?* — plus the ranked
hot-spots, for triaging which PRs deserve the closest read.

```bash
argot voice-diff main..HEAD              # metric + hot-spots for a range
argot voice-diff HEAD~5..HEAD --top 5    # show the 5 worst spots (default 10)
argot voice-diff main..HEAD --format json      # machine-readable summary
argot voice-diff main..HEAD --format markdown  # the GitHub score card (used by the Action)
```

`--format` accepts `human` (default), `json`, or `markdown` (the PR-comment / job-summary score
card). The metric is the smoothed proportion of hunks above threshold, so a tiny diff with one
anomalous hunk doesn't read 100%. It's pure aggregation over the same per-hunk scores `check`
produces — no extra modeling.

## inspect

Reports whether the repo is a good fit for argot (corpus composition, calibration health, a
Ready / Marginal / Not-recommended verdict), and can dump the fitted model artifact:

```bash
argot inspect                       # suitability verdict for the repo
argot inspect --format json         # the same, machine-readable
argot inspect --model               # the fitted model: hashes, provenance, typical callees per cluster
argot inspect --model --top 12      # show more typical callees per cluster
```

`argot fit` writes `.argot/manifest.json` — a versioned, hashed record of what argot learned (model
hash, scorer-config hash, fit commit + timestamp, corpus size). `inspect --model` reads it back and,
per language, lists the callees each cluster of your codebase leans on — a quick x-ray of the repo's
voice.

## describe-voice

Generate a human-readable **STYLE.md** from the learned voice — an onboarding companion that
describes how the repo *actually* writes (familiar imports, typical calls per area, red flags),
grounded in its own history rather than aspirational rules.

```bash
argot describe-voice                 # print the style guide to stdout
argot describe-voice --out STYLE.md  # write it to a file
argot describe-voice --top 12        # more typical callees per area
```

It's descriptive, not prescriptive: argot reports what the repo does. Feed it to a new contributor,
or hand it to an LLM agent as system-prompt context (the same signal the [MCP server](/docs/agents/)
serves programmatically).

## mute · list-mutes · review-mutes

The suppression commands — accept a reported hit for good, then review or prune what you've muted:

```bash
argot mute <hash> --reason "adopting axios repo-wide"   # append a [[mute]] to argot.toml
argot mute <hash> --reason "temporary" --expires 30d    # auto-expire after N days
argot list-mutes             # every active suppression, across all surfaces
argot review-mutes           # report hash-scoped mutes whose file is gone
argot review-mutes --prune   # …and rewrite the [[mute]] tables to drop the dead ones
```

`argot mute` records the exact file a hit came from, so `review-mutes` flags a
mute as **dead** once that file no longer exists in the working tree or `HEAD` —
the only point at which the mute can never fire again. `--prune` removes only
those, never a mute still guarding a file you have. `<hash>` is the
`[a1b2c3d4e5f6]` from a `check` hit line. The full suppression system — inline
comments, `argot.toml`'s `[exclude]`, and the `[[mute]]` format — is documented in
[Configure](/docs/configure/).

## mcp

Run a [Model Context Protocol](https://modelcontextprotocol.io) server over stdio, in-process against
the fitted `.argot/` model, so a coding agent can ask for the repo's voice while it works:

```bash
argot mcp --repo .           # serve voice_context / check / explain / fit_status over stdio
```

See [Agents](/docs/agents/) for the tools it exposes and how to wire it into Claude Code, Cursor, and
other MCP clients.

## status

The health hub — the answer to "is my setup still good?":

```text
Voice:    fitted at 4d488eb8b604 · fresh (nothing accepted since the fit)
Config:   in sync with the fit
Hygiene:  no unexcluded generated/data-heavy directories
```

`--format json` carries the same `health` block for scripts.


Show the current repository's argot state — whether it has an extracted dataset, a trained model, and
a calibrated threshold:

```bash
argot status                 # human summary for the current repo
argot status --format json   # the same, machine-readable
```

The JSON document carries `repo` (`name`, `path`), `dataset` (`records`, `bytes` — `null` if never
extracted), `model` (`trained`, `bytes`), and `calibrated` (bool).

## list

List every repository argot has been run in (tracked in `~/.argot/settings.json`), marking the current
one with `*`:

```bash
argot list                   # registered repos, current marked with *
argot list --format json     # repos[] with name, path, current
```

## update

Upgrade argot in place:

```bash
argot update
```

Self-update works for the curl-installer build (it reads the install receipt); an npm install prints
the `npm install -g @tmonier/argot@latest` command instead.

argot also nudges you passively: at most once a day it checks the published version file and prints
one dim stderr line when a newer release exists. It's silent in CI, on a non-tty, under `--quiet`,
and in machine formats, and it's opt-out — `ARGOT_UPDATE_CHECK=0` or `[update] check = false`. See
[Configure](/docs/configure/#update--the-passive-update-notice).

## uninstall

Leave as cleanly as you arrived. `uninstall` builds the full inventory of everything argot ever
wrote on the machine, shows it with sizes, and removes it after confirmation:

```bash
argot uninstall              # show the plan, confirm, remove
argot uninstall --dry-run    # just show the plan
argot uninstall --yes        # no prompt (required when not on a terminal)
```

It removes every registered repo's `.argot/` and `argot.local.toml`, the model cache
(`~/.cache/argot`), the global registry (`~/.argot/settings.json`), the installer receipt, and —
for curl/raw installs — the binary itself. It detects how argot was installed: an npm install gets
the exact `npm uninstall -g @tmonier/argot` command instead, since npm owns those files. Two things
are deliberately left, each listed with a note: **git-tracked files** (`argot.toml`, a committed CI
workflow) — argot never edits your tracked tree, remove those via git — and externally installed
agent skills / MCP registrations, which live in your agent's config, not argot's. The full file
inventory is the [table in Configure](/docs/configure/#which-files-argot-writes-and-where).

See [Reading the output](/docs/reading-the-output/) for how to interpret a `check` run.
