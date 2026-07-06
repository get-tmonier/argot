---
title: Getting started
description: Install argot and calibrate it on your repo in a couple of minutes.
group: Start
order: 1
---

**argot** is a voice linter. It learns your repo's voice from its own git history, then flags the
hunks whose token distribution diverges from the learned norm — the code that's valid, typed, and
lint-clean, but doesn't sound like anyone on your team wrote it. No model, no cloud, no GPU.

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
- It **does** answer the second question — *"is this how this team writes things?"* — the one that
  used to live in code review.

If your team ships LLM-assisted code, this is the layer your CI is missing.

## Where to next

- [Setup](/docs/setup/) — configure what argot should (and shouldn't) learn from.
- [How it works](/docs/how-it-works/) — the two-phase pipeline, in plain terms.
- [The commands](/docs/the-commands/) — `init`, `check`, and the rest in detail.
- [Reading the output](/docs/reading-the-output/) — severity tiers, sources, and the evidence line.
- [What it catches](/docs/what-it-catches/) — three real examples every other tool stays silent on.
