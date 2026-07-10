---
title: CI
description: A non-blocking voice score on every PR — a visual score card, a sticky comment, and code-scanning annotations. Never a merge gate unless you ask.
group: Guide
order: 9
---

argot in CI is **non-blocking by design**. It's a statistical guardrail, so it
*informs* a pull request — a visual score, the hot-spots, and inline annotations
— without ever gating the merge. The reviewer has the last word. (Want a hard
gate anyway? One input flips it on.)

This is the **CI path** and it's self-contained: **you don't need to install or
set up argot locally** — the Action installs it and fits the model in CI. Prefer
to run it on your own machine instead? See [Setup](/docs/setup/). You can do
both.

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
      - uses: get-tmonier/argot@main
```

argot fits the model on the PR's **base** branch and scores your changes against
it, so a dependency the PR introduces is judged as new (not learned as normal
first). The model is cached per base commit and only re-fit when the base moves.

> **Warming the embedding model.** The semantic layer's ~100 MB code-embedding
> model (GGUF) is fetched on first use. To pre-warm it as an explicit step — with
> a hard, legible failure if the runner can't reach the network — run
> **`argot model fetch`** before the check. On top of that, cache the model
> directory (`~/.cache/argot/models`) with `actions/cache` so the download
> happens once, not per run (the Action's `cache: true` default already keeps
> the fitted `.argot/`, including `.argot/semantic-index.json`, keyed on the
> base commit):
>
> ```yaml
>       - uses: actions/cache@v4
>         with:
>           path: ~/.cache/argot/models
>           key: argot-model-v1
>       - run: argot model fetch    # pre-warm; fails loudly if the download can't happen
> ```
>
> If the model isn't available at check time (an offline or locked-down runner),
> the semantic rules are skipped with a printed note and the **base
> foreign-catch guardrail still runs** — you never get a red build because a
> model download was blocked. On a locked-down mirror, set `ARGOT_MODEL_URL`
> (the sha256 is still verified) — see
> [Configure](/docs/configure/#environment-variables).

> **Committing the workflow:** pushing a `.github/workflows/*.yml` needs the
> `workflow` token scope. If `git push` is rejected with *"refusing to allow an
> OAuth App to … workflow … without 'workflow' scope"*, run
> `gh auth refresh -s workflow` (or push over SSH).

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
> > **Informational — not a merge gate.**

## Let an AI agent wire it in

Prefer to hand it off? Paste this into Claude Code (or Cursor, Aider, any agent)
at your repo root — it's the CI counterpart to the [Setup](/docs/setup/) prompt:

```text
You are adding **argot** to this repository's CI — a non-blocking voice check on
every pull request. You do NOT need argot installed locally; the GitHub Action
installs and fits it. Keep it informational — never a merge gate.

1. Confirm the repo is on GitHub with Actions enabled.

2. Create `.github/workflows/argot.yml`:
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
             fetch-depth: 0
         - uses: get-tmonier/argot@main

3. If the repo already has an `argot.toml`, leave it — the Action respects it.
   It is optional; don't invent one. (On a monorepo with peripheral packages,
   running the local `argot-setup` flow first to commit a good `argot.toml`
   makes the CI voice sharper, but isn't required.)

4. Commit and push the workflow. Pushing a `.github/workflows/*.yml` needs the
   `workflow` token scope — if `git push` is rejected, run
   `gh auth refresh -s workflow` (or push over SSH).

5. Do NOT add `fail-on-hits: true` unless I ask. Then tell me: on each PR I'll
   get a non-blocking voice-score card (PR comment + Actions job summary) and
   code-scanning annotations; it never fails the build.
```

(In Claude Code this is the **argot-setup-ci** skill — `npx skills add get-tmonier/argot`.)

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
      - uses: get-tmonier/argot@main
        with:
          fail-on-hits: true
```

Which findings fail is the rules engine's call: every rule defaults to severity
`error`, and anything the repo's `argot.toml` downgrades to `warn` is reported
without failing. In a hand-rolled workflow, `argot check --error-on-warnings`
turns even the `warn`-severity findings into a red build — the strictest
setting. See [Configure](/docs/configure/#rules--rule-severities).

## Inline PR annotations without the Action

In any hand-rolled workflow, `--format github` emits GitHub Actions workflow
commands (`::error file=…,line=…::message`) directly — the runner turns them
into inline PR annotations with **no SARIF upload step and no extra
permissions**:

```yaml
      - run: argot check origin/${{ github.base_ref }}..HEAD --format github
```

Each annotation carries the rule name, the score and confidence, the evidence,
and the exact `argot mute <hash>` command. `error`-severity rules annotate as
errors, `warn`-severity ones as warnings.

### Action inputs

All inputs are optional.

| Input | Default | What it does |
|---|---|---|
| `path` | `.` | Repository to check (working-directory relative). |
| `argot-version` | `latest` | Release to install — `latest` or a version like `0.2.48`. |
| `format` | `sarif` | Output format for `argot check`: `sarif`, `json`, or `human`. |
| `output-file` | `argot-results.sarif` | File the check results are written to. |
| `ref` | *(empty)* | Ref or range to check (e.g. `origin/main..HEAD`). Empty = automatic: `base..HEAD` on PRs, the head commit on pushes. |
| `cache` | `true` | Cache the fitted `.argot/` model between runs, keyed on the base commit. |
| `upload-sarif` | `true` | Upload the SARIF file to code scanning (needs `format: sarif` and `security-events: write`). |
| `comment-pr` | `true` | Post/update the sticky voice-score PR comment (needs `pull-requests: write`). |
| `fail-on-hits` | `false` | Fail the job when argot finds hits above the threshold. Off by default — argot informs without gating. |

### Action outputs

| Output | Meaning |
|---|---|
| `exit-code` | Exit code of `argot check` (`0` clean, `1` hits found). |
| `results-file` | Path to the written results file. |

## Locally, before you push

A [pre-commit](https://pre-commit.com) hook scores staged changes (informational —
it doesn't fail the commit unless you make it):

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/get-tmonier/argot
    rev: main   # pin to a release tag (e.g. v0.2.48) once you've picked one
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
argot check main..HEAD --format github     # inline PR annotations, no upload step
argot voice-diff main..HEAD --format markdown   # the score card
```

Exit codes: `0` clean · `1` at least one `error`-severity finding · `2`
setup/usage error. Treat `1` as "there's something to look at," not a failure —
that's the whole posture. Rules you've set to `warn` never exit 1 (unless you
pass `--error-on-warnings`).
