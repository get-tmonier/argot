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

## update

Upgrade argot in place:

```bash
argot update
```

See [Reading the output](/docs/reading-the-output/) for how to interpret a `check` run.
