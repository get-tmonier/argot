---
title: CI and pre-commit
description: Wire Argot into a GitHub workflow or commit hook with the default advisory behavior made explicit.
group: Configure
order: 4
---

CI and pre-commit are user-wired integrations: they run only after you add their configuration.
They report the selected changes at the workflow or commit event you configure; findings do not
decide what work is accepted, and the default behavior differs between the Action and the two
available pre-commit hooks.

## GitHub Action

```yaml
name: argot
on:
  pull_request:
  push:
    branches: [main]   # the run that refreshes the fitted artifacts PRs reuse

permissions:
  contents: read
  pull-requests: write     # optional sticky PR comment
  security-events: write   # optional SARIF upload

jobs:
  voice:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: get-tmonier/argot@main
```

The composite Action needs checkout history, release-download access, and the permissions needed
for whichever optional outputs you enable. It fits the base ref for a pull request and scores the
selected base-to-HEAD range, so the pull request’s code is not learned as the baseline. Its default
`fail-on-hits` is `false`: findings are reported without failing the job. When a team deliberately
sets it to `true`, error-severity results mark that Action job as failed; the team's review policy
still determines the response.

`format`, `ref`, `cache`, `semantic`, `upload-sarif`, and `comment-pr` are configurable Action
inputs. The semantic layer needs no network — its embedder ships in the binary — so a locked-down
runner needs no special handling; `semantic: false` is there to trade its cost away, not to work
around a download. The Action caches fitted artifacts by base commit when caching is enabled.

### Why a run took minutes

Fitting the base is almost the whole cost of an Action run — the check itself is seconds. On a cache
hit there is no fit at all, so the job is fast; on a miss it refits from scratch. The job summary says
which of the two happened, and how long the fit took.

### What a run costs

Fitting the voice model is almost the whole cost of a run; the check itself takes seconds. So the
Action splits the two:

- **A run on your default branch is the producer.** It fits and publishes the resulting `.argot/`
  artifacts into a cache slot. This is the run that costs the fit, after a merge, on nobody's
  critical path.
- **A pull request is a consumer.** It reads that slot and **does not fit** — the check is seconds.
  The job summary reports how many accepted commits the model is behind, which is the same drift
  argot tolerates locally between background refreshes (`[fit] refresh-after`).

That is why the workflow above triggers on `push` to the default branch as well as on
`pull_request`. Drop the `push` trigger and every pull request pays the fit instead.

**The cache does not exist until this workflow is merged.** The producer is a `push` to the default
branch, so while the pull request that *adds* the workflow is still open there is no slot to read:
that run, and any pull request opened before it lands, pays a cold fit rather than only the check.
This is the expected shape of the first day, not a sign the check is slow. Merge the
workflow, let the next default-branch push fill the slot, and pull requests drop to seconds.

A pull request refits in only two cases: no model exists yet — the first run on a repository, or the
slot expired after seven idle days, and the summary says it seeded the cache — or the base's
`argot.toml` changed since the model was fitted, which is a scope change rather than staleness.
`cache: false` disables the slot entirely and fits every run, if you want the base's exact model and
are willing to pay for it.

## pre-commit

The published hooks require `argot` on `PATH`, a fitted repository, a pre-commit configuration, and
`pre-commit install`. Both score staged supported files only.

```yaml
repos:
  - repo: https://github.com/get-tmonier/argot
    rev: v0.2.89
    hooks:
      - id: argot-check       # advisory for findings; operational errors still fail
      # - id: argot-check-gate  # opt-in: preserve error-severity exit status
```

`argot-check` turns exit 0 and finding exit 1 into a successful hook result; an unfitted repository
or command failure still fails. `argot-check-gate` preserves normal `argot check --staged` exit
semantics, so an error-severity result makes the hook fail. Remove the hook from
`.pre-commit-config.yaml` and run `pre-commit uninstall` when you no longer want it installed.

For another CI provider, explicitly run `argot fit` against the chosen baseline and then
[`argot check`](/docs/check/) against the chosen range. Decide and document your own exit-code
policy; no host runs this command unless its workflow is configured to do so.
