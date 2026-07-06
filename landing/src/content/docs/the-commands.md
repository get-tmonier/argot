---
title: The commands
description: init and check — the everyday commands — plus fit, extract, and the on-demand tools.
group: Guide
order: 4
---

The two everyday commands are **`argot init`** (one-time setup — it fits the model and health-checks
the repo) and **`argot check`** (the per-diff loop). `fit` is what `init` runs under the hood;
`extract` writes a raw training dataset the check path doesn't need, so most repos never run it. The
rest — `review`, `voice-diff`, `inspect`, `mute` — are on demand. Run `argot --help` for the full list.

## init

Fits the voice model to the repo (`fit`), prints a health check (corpus composition + a
Ready / Marginal / Not-recommended verdict), and writes a `.argot/.gitignore` so the rebuildable
model stays out of version control. This is the one command a new repo needs.

```bash
argot init                   # set up the current repo
argot init --suggest         # list generated/data-heavy dirs you may want to exclude first
argot init --suggest --format json   # the same, machine-readable (for the setup skill)
```

See [Setup](/docs/setup/) for deciding what shouldn't shape your voice.

## extract

Walks the repo's git history and writes a training dataset — one record per hunk, with tokenized
context and content.

```bash
argot extract                # full history of the current repo
argot extract HEAD~50        # history up to and including HEAD~50
argot extract main..HEAD     # only commits in that range
argot extract --limit 5000   # stop after 5000 records
argot extract --out data.jsonl   # write somewhere other than the default
```

The current repo is auto-detected. Flags: `--repo <path>` (default `.`), `--out <path>` (default
`.argot/dataset.jsonl`), `--limit <N>` (cap the records emitted).

## fit

One-shot voice fitting: collects the repo's source files as the repo corpus, sets up the generic
baseline, then samples representative hunks to set the scoring threshold.

```bash
argot fit
```

Writes three artifacts under `.argot/`:

| File | What it is |
|---|---|
| `repo-corpus.txt` | the source files counted into the repo distribution |
| `generic-baseline.json` | the bundled generic baseline reference |
| `scorer-config.json` | the calibrated threshold(s) and scorer config |

It also refreshes `.argot/manifest.json` (the hashed model record). For every file argot writes,
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

Scores changed hunks against the trained scorer and prints them grouped by file.

**Exit codes:** `0` clean · `1` hits found — *something to look at, not a failure* · `2` setup/usage
error. For CI, prefer the non-blocking [GitHub Action](/docs/ci/) over a hand-rolled
`argot check || fail`: because a foreign import can land in any tier, gating the CLI exit code turns
every advisory hit — down to `unusual` — into a red build. The Action posts an advisory score card
instead, and only blocks if you explicitly ask it to.

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
argot check --only 'src/*'          # restrict to matching files (repeatable)
argot check --exclude 'test/*'      # drop matching files (repeatable; wins over --only)
argot check --min-severity foreign  # only show foreign-tier hits
argot check --verbose               # show full hunk contents (no truncation)
```

`--min-severity` filters by tier. Keep the default (`unusual`) to see everything argot flags — a lone
foreign import can score right at the threshold and land in `unusual`, so `--min-severity foreign`
(the strongest-anomaly tier) may *hide* a single new dependency. Raise it to `suspicious` or `foreign`
only to cut noise on a chatty repo, once you trust the calibration.

Every `check` run also names the model that judged the diff — a short `model:` hash on stderr (human)
or in the `model` field of `--format json`/`sarif`. Same corpus + config always fits the same hash, so
you can tell at a glance whether your model matches a colleague's.

### Output and advanced flags

```bash
argot check --format json           # stable machine JSON (human | json | sarif; default human)
argot check --format sarif          # SARIF 2.1.0 for code-scanning uploads
argot check --hunk-lines 12         # lines of hunk body under each hit (default 6; 0 to suppress)
argot check --repo ../other-repo    # check a repo other than the current directory (default .)
```

| Flag | Default | What it does |
|---|---|---|
| `--format` | `human` | `human`, `json` (stable schema), or `sarif` (SARIF 2.1.0). Machine formats write only the document to stdout — see [Reading the output](/docs/reading-the-output/). |
| `--repo <path>` | `.` | Repository to check. |
| `--argot-dir <path>` | `.argot` | Where to load the fitted model from. A relative path is resolved against `--repo`; an absolute path is used verbatim. |
| `--hunk-lines <N>` | `6` | Hunk-body lines shown under each hit (`0` suppresses them; `--verbose` overrides with the full hunk). |

Color follows the [`NO_COLOR`](https://no-color.org) convention: argot colors severity markers only when
`NO_COLOR` is unset **and** stdout is a terminal. Machine formats are never colored.

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
straight through to the local git — no network. `--format json|sarif` works the
same as `check`. `review` also prints a one-line **voice-diff** headline above
the hits.

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
argot mute <hash> --reason "adopting axios repo-wide"   # append a rule to .argot/suppressions.yaml
argot mute <hash> --reason "temporary" --expires 30d    # auto-expire after N days
argot list-mutes             # every active suppression, across all three surfaces
argot review-mutes           # report hash-scoped mutes whose file is gone
argot review-mutes --prune   # …and rewrite suppressions.yaml to drop the dead ones
```

`argot mute` records the exact file a hit came from, so `review-mutes` flags a
mute as **dead** once that file no longer exists in the working tree or `HEAD` —
the only point at which the mute can never fire again. `--prune` removes only
those, never a mute still guarding a file you have. `<hash>` is the
`[a1b2c3d4e5f6]` from a `check` hit line. The full suppression system — inline
comments, `.argotignore`, and the `suppressions.yaml` format — is documented in
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

See [Reading the output](/docs/reading-the-output/) for how to interpret a `check` run.
