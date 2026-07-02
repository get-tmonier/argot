---
title: The commands
description: extract, fit, and check — what each one does and the flags that matter.
group: Guide
order: 3
---

argot is three commands. `extract` and `fit` are one-time setup; `check` is the per-diff loop.

## extract

Walks the repo's git history and writes a training dataset — one record per hunk, with tokenized
context and content.

```bash
argot extract                # full history of the current repo
argot extract HEAD~50        # history up to and including HEAD~50
argot extract main..HEAD     # only commits in that range
```

The current repo is auto-detected via `git rev-parse`. Output: `.argot/dataset.jsonl`.

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

Re-run `fit` after a major refactor. Internally it runs the engine's two underlying phases (build
corpus, then calibrate); both stay available as engine entry points for benchmark and research use.

## check

Scores changed hunks against the trained scorer, prints them grouped by file, and **exits non-zero**
if any hunk is above the calibrated threshold — so it drops straight into CI.

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

`--min-severity` is the dial you'll reach for most: start at `foreign` for high-confidence anomalies,
loosen to `suspicious` once you trust the calibration on your repo.

Every `check` run also names the model that judged the diff — a short `model:` hash on stderr (human)
or in the `model` field of `--format json`/`sarif`. Same corpus + config always fits the same hash, so
you can tell at a glance whether your model matches a colleague's.

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
same as `check`.

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

## update

Upgrade argot in place:

```bash
argot update
```

See [Reading the output](/docs/reading-the-output/) for how to interpret a `check` run.
