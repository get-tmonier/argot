---
title: Init and Fit
description: Set up portable configuration and a committed fit snapshot with init; refresh it locally with fit.
group: Configure
order: 1
---

Use `argot init` for first-time repository setup. When maintenance is recommended, use the
`argot-refresh` skill to re-audit corpus paths and mutes before it deliberately runs `argot fit`.

```bash
argot init                 # create portable configuration, fit, and report health
argot init --suggest       # suggestions only; does not fit or edit configuration
argot init                 # refit after any reviewed configuration change
argot fit                  # rebuild local artifacts only
argot inspect              # inspect corpus composition and health
argot inspect --corpus     # list the files that will shape the voice — before any fit
```

`inspect` describes the corpus the fit will actually use: gitignored trees are named, never
counted, so a dependency directory can never masquerade as part of your voice. `--corpus` prints
that file list outright, which is the fastest way to check an exclusion decision **before**
committing it.

## What each command writes

`init` fits `.argot/`, creates a commented root `argot.toml` when it is missing, and adds
`argot.local.toml` to the root `.gitignore` for personal overrides. It also writes a selective
`.argot/.gitignore`: the fit snapshot is visible to Git, while caches and one-run state remain local.

`fit` deliberately does **not** create `argot.toml` or edit the root `.gitignore`. It refreshes the
reviewable artifacts under `.argot/`; inspect their diff and commit them. `fit` can be used on
detached checkouts, which is why configuration scaffolding belongs only to `init`.

For a repository using Argot, commit the reviewed `argot.toml` **and the fit snapshot under
`.argot/`**. The embedder itself remains compiled into Argot's binary; the committed files are your
repository's learned voice, semantic index, layering graph, integrity model, and provenance. CI
only reads this snapshot — it never fits or rebuilds it.

## Build a trustworthy voice

Run `argot init` first on a fresh clone so its shared `argot.toml` exists. Then run
`argot init --suggest`, review the proposed directories — the generated and data-heavy ones, and
the ones your repository stores but never writes (vendored libraries, forked upstream copies), each
reported with the evidence behind it — edit `argot.toml [exclude].paths` only when you agree, and
run `argot init` again. Exclusions shape the voice; they are not a way to silence ordinary findings.

Manual `init` and `fit` learn the files on disk. Prefer a clean checkout of the default branch:
both commands warn if uncommitted source files or unmerged source commits would be learned. A
manual fit still runs after that warning, so the choice remains yours. Set
`[fit] refresh-from = "current-branch"` only when branch fitting is intentional.

`argot status` measures final accepted source/function/layout drift from `fit_sha`; commit count and age do
not trigger maintenance by default. Its structured `refresh.next_action` is `fit` for ordinary
content drift and `review_scope_then_fit` when new/moved paths, a language surface, layout, or
fit-relevant configuration should be audited first. `argot-refresh` also runs the mute review
read-only and asks once before editing policy. Neither status, check, the skill, nor CI refits in
the background: fitting is a deliberate local step whose `.argot/` diff is reviewed and committed.
See [Snapshot health and refresh](/docs/health-and-freshness/) for the measured signals, structured
reasons, and complete maintenance flow.

## Adopting on an existing codebase

A repository with history will have findings the day you fit it. Decide which
starting line you want before wiring argot into anything:

- **Baseline** — `argot check --add-ignores` writes an inline ignore above every finding that
  exists today, so only new code is judged from here. Right for a mature codebase adopting argot
  without a cleanup project first. Those ignores are a snapshot: re-score them periodically, or the
  baseline quietly becomes permanent.
- **Clean slate** — fix or mute the existing findings now. Right for a smaller or younger repository.

## Health

`argot inspect` reports `Ready`, `Ready with notes`, or `Not recommended`. Treat notes as tuning
evidence, and down-weight findings if the fit is not recommended. `--format json` makes the verdict
and its reasons machine-readable, so a setup flow can gate on them rather than eyeball a line.

One reason is worth acting on immediately: `voice_not_where_the_work_is` means a directory shapes a
large share of the voice while taking almost none of the recent changes — a model learned from code
nobody edits, judging the code everybody does. It names the directory and both shares. If that tree
is demos, vendored, or generated, exclude it and refit. The semantic index is built with a model
compiled into the binary — no download, no cache to warm, and a fit on an air-gapped machine
behaves exactly like one online.

For configuration syntax and artifact reference details, see [Configure](/docs/configure/).
