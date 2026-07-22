---
title: Init and Fit
description: Set up portable configuration with init; refresh only local model artifacts with fit.
group: Configure
order: 1
---

Use `argot init` for first-time repository setup. Use `argot fit` when you intentionally want to
rebuild the local model after configuration or source layout changes.

```bash
argot init                 # create portable configuration, fit, and report health
argot init --suggest       # suggestions only; does not fit or edit configuration
argot init                 # refit after any reviewed configuration change
argot fit                  # rebuild local artifacts only
argot inspect              # inspect corpus composition and health
```

## What each command writes

`init` fits `.argot/`, creates a commented root `argot.toml` when it is missing, and adds
`argot.local.toml` to the root `.gitignore` for personal overrides. Commit the shared `argot.toml`
when its exclusions and rule choices represent a team decision.

`fit` deliberately does **not** create `argot.toml` or edit the root `.gitignore`. It writes
rebuildable artifacts under `.argot/`, including the fitted scorer configuration and indexes;
`.argot/.gitignore` protects that directory from version control. `fit` can be used on detached
checkouts, which is why configuration scaffolding belongs only to `init`.

## Build a trustworthy voice

Run `argot init` first on a fresh clone so its shared `argot.toml` exists. Then run
`argot init --suggest`, review the proposed generated or data-heavy directories, edit
`argot.toml [exclude].paths` only when you agree, and run `argot init` again. Exclusions shape the
voice; they are not a way to silence ordinary findings.

Manual `init` and `fit` learn the files on disk. Prefer a clean checkout of the default branch:
both commands warn if uncommitted source files or unmerged source commits would be learned. A
manual fit still runs after that warning, so the choice remains yours. Set
`[fit] refresh-from = "current-branch"` only when branch fitting is intentional.

Each `check` can schedule a background refresh when the accepted-history fit is stale. That refresh
uses an accepted-history anchor and avoids dirty or unmerged source changes; it can be disabled with
`[fit] auto-refresh = false`. It does not replace a deliberate refit after you change exclusions.

## Health and offline use

`argot inspect` reports `Ready`, `Ready with notes`, or `Not recommended`. Treat notes as tuning
evidence, and down-weight findings if the fit is not recommended. The semantic index uses a local
embedding model that may download once to a machine cache. Run `argot model fetch` before going
offline, or use the offline configuration where semantic checks are unavailable.

For configuration syntax and artifact reference details, see [Configure](/docs/configure/).
