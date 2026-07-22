---
title: CI and pre-commit
description: Wire Argot into a GitHub workflow or commit hook with the default advisory behavior made explicit.
group: Configure
order: 4
---

CI and pre-commit are user-wired integrations: they run only after you add their configuration.
They are not acceptance-time checks, and the default finding behavior differs between the Action
and the two available pre-commit hooks.

## GitHub Action

```yaml
name: argot
on: pull_request

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
`fail-on-hits` is `false`: findings are reported without failing the job. Set it to `true` only
when you intentionally want error findings to gate the workflow.

`format`, `ref`, `cache`, `semantic`, `upload-sarif`, and `comment-pr` are configurable Action
inputs. Semantic checking may download the local embedding model; use `semantic: false` on a
locked-down or offline runner to keep voice, layering, and integrity checks while skipping semantic
model work. The Action caches fitted artifacts by base commit when caching is enabled.

## pre-commit

The published hooks require `argot` on `PATH`, a fitted repository, a pre-commit configuration, and
`pre-commit install`. Both score staged supported files only.

```yaml
repos:
  - repo: https://github.com/get-tmonier/argot
    rev: v0.2.89
    hooks:
      - id: argot-check       # advisory for findings; operational errors still fail
      # - id: argot-check-gate  # opt-in: error findings reject the commit
```

`argot-check` turns exit 0 and finding exit 1 into a successful hook result; an unfitted repository
or command failure still fails. `argot-check-gate` preserves normal `argot check --staged` exit
semantics and rejects error-severity findings. Remove the hook from `.pre-commit-config.yaml` and
run `pre-commit uninstall` when you no longer want it installed.

For another CI provider, explicitly run `argot fit` against the chosen baseline and then
[`argot check`](/docs/check/) against the chosen range. Decide and document your own exit-code
policy; no host runs this command unless its workflow is configured to do so.
