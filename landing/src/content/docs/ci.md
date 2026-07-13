---
title: CI
description: A non-blocking voice score on every PR — a visual score card, a sticky comment, and code-scanning annotations. Never a merge gate unless you ask.
group: Guide
order: 10
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

> **What the embedding model costs in CI — and what the Action already does.**
> The semantic layer's ~100 MB (104 MB) GGUF is fetched on first use. The
> Action handles it for you: it caches `~/.cache/argot/models` under a key tied
> to the model's release tag (so the download happens **once per repo × OS**,
> not per run — the tag only changes when a release pins a new model) and runs
> `argot model fetch` (a cache hit costs one sha256 pass, well under a second;
> a cold download from GitHub Releases is typically 10–30 s on hosted runners).
> The heavier, easy-to-miss cost is the **fit-time semantic index**: fitting on
> a new base embeds every corpus function on a CPU runner — minutes on a large
> repo. The Action's `cache: true` keeps the fitted `.argot/` (index included)
> keyed on the base commit, so you pay it only when the base advances — and on
> that advance it restores the *previous* base's index first, so the re-fit is
> incremental (unchanged functions reuse their embeddings; seconds, not
> minutes). As a second safety net it also caches
> `~/.cache/argot/embeddings` — the machine-wide, content-addressed embedding
> cache — under a loose restore-key, so even a fresh runner or a cache miss on
> `.argot/` re-embeds only the functions that actually changed. If that
> is still too much — or the runner is locked down — set the Action's
> **`semantic: false`** input: no download, no index build; the voice,
> layering, and integrity rules still run — they're pure Rust, no model
> download.
>
> Hand-rolled workflow (no Action)? Reproduce the same steps — cache the model
> (immutable, keyed on its tag) and the embedding vectors (incremental, loose
> restore-key):
>
> ```yaml
>       - uses: actions/cache@v4        # the ~100 MB model
>         with:
>           path: ~/.cache/argot/models
>           key: argot-embedding-model-semantic-model-v1-${{ runner.os }}
>       - uses: actions/cache@v4        # the machine-wide embedding cache
>         with:
>           path: ~/.cache/argot/embeddings
>           key: argot-embeddings-${{ runner.os }}-${{ github.sha }}
>           restore-keys: |
>             argot-embeddings-${{ runner.os }}-
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
| `argot-version` | `latest` | Release to install — `latest` or a version like `0.2.59`. |
| `format` | `sarif` | Output format for `argot check`: `sarif`, `json`, `github`, or `human`. |
| `output-file` | `argot-results.sarif` | File the check results are written to. |
| `ref` | *(empty)* | Ref or range to check (e.g. `origin/main..HEAD`). Empty = automatic: `base..HEAD` on PRs, the head commit on pushes. |
| `cache` | `true` | Cache the fitted `.argot/` model between runs, keyed on the base commit. |
| `upload-sarif` | `true` | Upload the SARIF file to code scanning (needs `format: sarif` and `security-events: write`). |
| `comment-pr` | `true` | Post/update the sticky voice-score PR comment (needs `pull-requests: write`). |
| `rules` | *(empty)* | Space-separated rule severity overrides passed to `argot check --rule`, e.g. `misplaced=warn semantic=off`. Empty = the repo's `argot.toml` `[rules]`. |
| `semantic` | `true` | Run the semantic rules (`redundant`/`misplaced`). `false` sets `ARGOT_OFFLINE=1` — no model download, no index build; the voice, layering, and integrity rules still run. |
| `fail-on-hits` | `false` | Fail the job when argot finds hits above the threshold. Off by default — argot informs without gating. |

### Action outputs

| Output | Meaning |
|---|---|
| `exit-code` | Exit code of `argot check` (`0` clean, `1` hits found, `2` setup/usage error). |
| `results-file` | Path to the written results file. |

## On any other CI — GitLab, Jenkins, CircleCI, …

The Action is a convenience, not a requirement. On any provider, a voice check
is four steps — and two caches:

1. **Install the binary** (~20 MB, pin a version for reproducible runs):
   `curl -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh`,
   then make sure `~/.local/bin` / `~/.cargo/bin` is on `PATH`.
2. **Warm the embedding model** — cache the models directory and run
   `argot model fetch` (cache hit: one sha256 pass, <1 s; cold: one ~104 MB
   download). Locked-down runner? `ARGOT_OFFLINE=1` skips it — voice,
   layering, and integrity still run.
3. **Fit on the base, not the head** — check out the target branch, `argot
   fit`, check out the PR head again. Fitting on the head would teach the
   model the PR's own new code before judging it. Cache `.argot/` keyed on
   the base commit so the fit (and its semantic index) re-runs only when the
   base advances.
4. **Check the range** — `argot check "origin/$TARGET..HEAD" --format json`
   (exit 0 clean · 1 findings · 2 setup error; add `--error-on-warnings` for
   a strict gate).

GitLab CI, as one concrete shape (GitLab only caches paths inside the project
dir — point `XDG_CACHE_HOME` there so the model cache is cacheable):

```yaml
argot:
  variables:
    GIT_DEPTH: 0                                # ranges need history
    XDG_CACHE_HOME: $CI_PROJECT_DIR/.cache      # model cache inside the project dir
  cache:
    - key: argot-model-semantic-model-v1
      paths: [.cache/argot/models]
    - key: argot-fit-$CI_MERGE_REQUEST_DIFF_BASE_SHA
      paths: [.argot]
  script:
    - curl -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh
    - export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
    - argot model fetch
    - |
      if [ ! -f .argot/scorer-config.json ]; then
        git checkout --detach "$CI_MERGE_REQUEST_DIFF_BASE_SHA"
        argot fit
        git checkout --detach "$CI_COMMIT_SHA"
      fi
    - argot check "$CI_MERGE_REQUEST_DIFF_BASE_SHA..HEAD"
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

Two behaviours you get for free on runners: the background auto-refit and the
update notice **never run in CI** (argot detects `CI`, and Jenkins/TeamCity/
Azure markers too), and a blocked model download degrades to a printed skip —
never a red build.

## Locally, before you push

A [pre-commit](https://pre-commit.com) hook scores staged changes (informational —
it doesn't fail the commit unless you make it):

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/get-tmonier/argot
    rev: main   # pin to a release tag (e.g. v0.2.59) once you've picked one
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
