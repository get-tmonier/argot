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
    branches: [main]

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
for whichever optional outputs you enable. It reads the **committed fit snapshot** from the base ref
for a pull request and scores the selected base-to-HEAD range, so the pull request’s code — or a
snapshot it edits — is not learned as the baseline. Its default
`fail-on-hits` is `false`: findings are reported without failing the job. When a team deliberately
sets it to `true`, error-severity results mark that Action job as failed; the team's review policy
still determines the response.

`format`, `ref`, `semantic`, `upload-sarif`, and `comment-pr` are configurable Action
inputs. The semantic layer needs no network — its embedder ships in the binary — so a locked-down
runner needs no special handling; `semantic: false` is there to trade its cost away, not to work
around a download.

### The committed snapshot contract

Before enabling CI, run `argot init` locally on the accepted branch, review and commit `argot.toml`
and the fit snapshot under `.argot/`. The snapshot includes the voice scorer, generic baseline,
semantic index, layering/integrity artifacts, health record, and manifest; transient caches are
ignored. The Action never runs `argot fit`, restores no cache, and never writes a snapshot.

On every PR it extracts the base commit’s snapshot into a temporary directory. That prevents a PR
from self-certifying by changing `.argot/`. Its summary reports how many accepted source commits the
snapshot is behind and tells the team to refresh it locally when due. A missing, incomplete, or
configuration-mismatched base snapshot is a clear setup error rather than a silent partial check.

### Refreshing it

When the scorecard says the fit is behind, update the accepted branch locally:

```sh
argot fit
argot status
git add .argot/ argot.toml
git commit -m "chore(argot): refresh fit snapshot"
```

Do this after accepted code or a scope/configuration change, not on a feature branch whose code the
fit should still judge. The Action remains fast because a check only reads committed files.

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

For another CI provider, install Argot, read the committed base snapshot, then run
[`argot check`](/docs/check/) against the chosen range. Do not run `argot fit` in CI; refresh and
commit the snapshot locally instead. Decide and document your own exit-code policy.
