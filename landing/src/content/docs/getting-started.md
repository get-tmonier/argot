---
title: Getting started
description: Install argot and calibrate it on your repo in a couple of minutes.
group: Start
order: 1
---

**argot** learns your repo's patterns from its own git history, then flags code **foreign to your
codebase** — a dependency, API, or whole construct it has never used. It's the "unknown to this
repo" code an AI agent reaches for when it doesn't know your stack: valid, typed, and lint-clean,
but not how anything here is actually built. No model, no cloud, no GPU.

> **Status: alpha.** argot is a probabilistic style linter — treat every flag as a prompt to look,
> and verify before you gate CI on it. It ships honest, leak-free benchmarks and a public research log.

## Install

argot is a **single static binary** — no Python, no Node, nothing else to install.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh
```

Prefer a package manager?

```bash
npm install -g @tmonier/argot
```

Both download the prebuilt `argot` binary for your platform (macOS arm64/Intel, Linux x64/arm64).
Everything runs locally — no API key, no account, nothing leaves your machine.

## Two ways to run it — pick one, or both

- **On your machine** — check as you work (and in a pre-commit hook). Install the
  CLI, `argot init`, then `argot check`. Start below.
- **In CI** — a non-blocking voice score on every pull request. Just add a GitHub
  Actions workflow; **you don't need to set argot up locally for this** — the
  Action installs and fits it for you. See [CI](/docs/ci/).

They're independent: do the local flow, the CI flow, or both. The rest of this
page is the local flow.

## Set up locally, then check

The fastest path is one command:

```bash
cd your-repo
argot init         # learn your repo's voice, then a health check (Ready / Marginal / …)
argot check        # score uncommitted changes (or pass a ref/range)
```

`argot init` fits the model once and writes a `.argot/.gitignore` so the rebuildable model stays out
of git. If the health check isn't **Ready**, or your repo has generated / vendored / peripheral code
(a monorepo's marketing site, playground, or demo apps), spend a minute on [Setup](/docs/setup/) to
tell argot what shouldn't shape its voice. Then run `check` on every diff.

```text
argot check · 2 hunks above threshold (1 foreign · 1 suspicious)
note: argot is a probabilistic style linter — verify before action.

src/utils/http-helpers.ts
  !  L42-L48   8.21  foreign     · workdir · foreign import (import) [a1b2c3d4e5f6]
     ↳ axios — 0 of 47 module specifiers in repo
       common here: react (320×), express (88×), pg (47×)
```

## What argot is — and isn't

- It **does not** replace ESLint, ruff, or your type checker. Those answer *"is this valid?"*
- It **reliably** catches what they can't articulate: a **foreign dependency, API, or paradigm** —
  something the repo has never used. When the foreign symbol is in the diff, it catches ~99% of them.
- It is **honest about its limit**: it does *not* reliably flag an *in-vocabulary* choice — a bare
  `ValueError` where you'd raise `HTTPException`, when every token is already yours. So a clean run
  means "no foreign pattern found," **not** "this matches every convention." argot never gates on
  those subtle cases.

If your team ships LLM-assisted code, this is the layer your CI is missing.

> **One setup note:** argot learns from your files *as they are on disk*, so fit on a **clean
> working tree** — commit or stash work in progress first, or uncommitted foreign code gets learned
> as normal. `argot init`/`fit` warns you when the tree is dirty.

## Where to next

- [Setup](/docs/setup/) — configure what argot should (and shouldn't) learn from.
- [Configure](/docs/configure/) — `.argotignore`, inline comments, and durable `argot mute`.
- [How it works](/docs/how-it-works/) — the two-phase pipeline, in plain terms.
- [The commands](/docs/the-commands/) — `init`, `check`, `fit`, `mute`, and the rest in detail.
- [Reading the output](/docs/reading-the-output/) — severity tiers, sources, and the evidence line.
- [What it catches](/docs/what-it-catches/) — three real examples every other tool stays silent on.
