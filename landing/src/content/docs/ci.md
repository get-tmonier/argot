---
title: CI
description: A non-blocking voice score on every PR — a visual score card, a sticky comment, and code-scanning annotations. Never a merge gate unless you ask.
group: Guide
order: 9
---

argot in CI is **advisory by design**. It's a statistical guardrail, so it
*informs* a pull request — a visual score, the hot-spots, and inline annotations
— without ever gating the merge. The reviewer has the last word. (Want a hard
gate anyway? One input flips it on.)

## GitHub Action

```yaml
# .github/workflows/argot.yml
name: argot
on: pull_request

permissions:
  contents: read
  pull-requests: write     # the sticky score comment
  security-events: write   # SARIF code-scanning annotations

jobs:
  voice:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0    # argot diffs the PR range, so it needs history
      - uses: get-tmonier/argot@v1
```

That's the whole setup. On each PR you get three things, none of which block the
merge:

1. **A score card in the Actions run** (job summary) — always, no extra
   permissions, works on forks.
2. **A sticky PR comment** — one comment that updates in place, showing the
   voice score and hot-spots, each with the exact `argot mute <hash>` to accept
   it. Needs `pull-requests: write`.
3. **Inline annotations** in the Security tab (SARIF code scanning). Needs
   `security-events: write`.

The card looks like this:

> ### 🎙️ argot voice check
> **83% in-voice** · 1 of 1 scored hunks look foreign to this repo's patterns · strongest signal: **suspicious**
>
> `█████████████████░░░` 83%
>
> > **Advisory — not a merge gate.**

## The human keeps the last word

The comment never says "fix" — it says *review*, and every hit shows how to
accept it:

```text
argot mute <hash> --reason "adopting axios repo-wide"
```

That mute is committed, so it's an audit trail of a deliberate decision, and the
hit never comes back. Nothing blocks the merge unless a maintainer chooses to
require the check.

## Opt into a hard gate

If you *do* want argot to fail the job on hits (e.g. a repo with a strict voice
policy):

```yaml
      - uses: get-tmonier/argot@v1
        with:
          fail-on-hits: true
```

Other inputs: `argot-version` (pin a release), `path`, `ref` (an explicit
range), `format`, `cache`, `upload-sarif`, `comment-pr`. All optional.

## Locally, before you push

A [pre-commit](https://pre-commit.com) hook scores staged changes (advisory —
it doesn't fail the commit unless you make it):

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/get-tmonier/argot
    rev: v1
    hooks:
      - id: argot-check
```

Run `argot fit` (or [`argot init`](/docs/setup/)) once first — the hook scores
against the fitted model.

## Machine-readable output

Every surface is built from the CLI's stable formats, so you can wire argot into
any system:

```text
argot check main..HEAD --format json      # stable JSON: hits, scores, hashes
argot check main..HEAD --format sarif      # SARIF 2.1.0 for any code scanner
argot voice-diff main..HEAD --format markdown   # the score card
```

Exit codes: `0` clean · `1` hits found · `2` setup/usage error. Treat `1` as
"there's something to look at," not a failure — that's the whole posture.
