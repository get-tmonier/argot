---
title: Snapshot health and refresh
description: Understand when Argot's committed repository memory needs maintenance, and refresh it locally without turning CI into a training system.
group: Configure
order: 5
---

Argot checks code against a **reviewed snapshot of what the repository had learned**. That snapshot
lives under `.argot/` and is committed with `argot.toml`, so developers, agents, and CI all use the
same baseline. It is learned state, not a disposable cache.

The embedding model itself is different: it already ships inside the Argot binary. Git stores only
your repository-specific voice, semantic index, architecture and integrity artifacts, plus the
health metadata that makes drift explainable.

## When does Argot recommend a refresh?

`argot status` compares the accepted tree with the compact source profile stored at `fit_sha`. The
calculation is deterministic and offline; it does not rerun fitting or embeddings. It measures:

- changed source lines as a share of the fitted corpus,
- changed function bodies as a share of the fitted function surface,
- added, deleted, or moved files as layout turnover,
- material drift inside a language or monorepo area,
- new material language or directory surfaces,
- and fit-relevant `[exclude]` or `[detect]` configuration changes.

It does **not** recommend maintenance merely because ten commits landed, a date passed, or docs
changed. A team can configure an explicit `[fit] refresh-after` backstop, but there is no commit or
age threshold by default.

## Read the recommendation and the route

```sh
argot status
argot status --format json
```

The human verdict is `fresh`, `watch`, `recommended`, or `strongly_recommended`. The structured
`refresh.next_action` tells a user or agent what kind of maintenance is appropriate:

| `next_action` | Meaning | Response |
| --- | --- | --- |
| `none` | No material accepted drift | Do nothing. |
| `monitor` | Early drift is visible | Keep working; no routine notification or fit is needed. |
| `fit` | Existing learned source/function surfaces materially changed | Use `argot-refresh`, then perform a reviewed local fit. |
| `review_scope_then_fit` | A new/moved area, language, layout, or fit-relevant config may have changed what belongs in the corpus | Review paths, exclusions, corpus composition, and mutes before fitting. |
| `inspect_history` | This clone cannot compare the fit with accepted history | Fetch or use a full accepted-branch clone; do not guess. |

Every recommendation carries structured reasons such as `source_turnover`,
`function_surface_turnover`, `layout_turnover`, `new_area_surface`, or
`new_language_surface`, with changed/baseline/current counts and a ratio. CLI JSON, MCP
`get_fit_status`, local checks, and the GitHub Action expose the same assessment.

## The complete refresh experience

Invoke the `argot-refresh` skill on a clean accepted/default branch. It performs one deliberate
maintenance pass:

1. **Diagnose.** Read compatibility, recommendation, reasons, and `next_action`.
2. **Re-audit scope read-only.** Run `argot init --suggest`, inspect the current corpus, resolve
   existing exclusion paths, and investigate new or moved areas.
3. **Review policy separately.** List and review mutes, expiries, standing path exceptions, rule
   severities, and migrations. Mutes do not train the model, but stale policy can hide evidence.
4. **Ask once.** Present every proposed scope or policy edit with its evidence and corpus/check
   effect. Apply only what the user confirms.
5. **Fit locally.** Run `argot fit`; never do this in CI or in the background.
6. **Verify and commit.** Require a complete, ready, fresh snapshot; review the exact
   `argot.toml`/`.argot/` diff and commit it as a small maintenance change.

```sh
argot init --suggest --format json
argot inspect --corpus
argot list-mutes
argot review-mutes
# approve any justified maintenance first
argot fit
argot status --format json
git add argot.toml .argot/
git commit -m "chore(argot): refresh fit snapshot"
```

If only mutes or rule policy changed and no model refresh was recommended, stop without fitting.
`[exclude]` and `[detect]` shape what Argot learns; mutes, `[rules]`, and migrations shape what a
check displays.

## Why CI only reads the base snapshot

For a pull request, the GitHub Action extracts the committed snapshot from the base ref and checks
the PR against it. A PR therefore cannot change its own learned baseline and certify itself. CI may
surface an advisory refresh notice for the accepted branch, but it never fits, caches, commits, or
rebuilds Argot artifacts.

This keeps the operational model intentionally small: one reviewed repository memory in Git, fast
checks everywhere, and an occasional local maintenance commit when the repository has genuinely
moved.

For artifact names and sizes, see [Configure](/docs/configure/#which-files-live-where). For initial
setup, see [Init and Fit](/docs/init-and-fit/). For symptom-based recovery, see
[Troubleshooting](/docs/troubleshooting/).
