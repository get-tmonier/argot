---
title: Getting started
description: Install argot and calibrate it on your repo in a couple of minutes.
group: Start
order: 1
---

**argot** is a voice linter. It learns your repo's voice from its own git history, then flags the
hunks whose token distribution diverges from the learned norm — the code that's valid, typed, and
lint-clean, but doesn't sound like anyone on your team wrote it. No model, no cloud, no GPU.

> **Status: alpha.** The scorer works and ships honest benchmarks, but argot is not yet recommended
> for production gating. See [Limitations](/docs/limitations/) for the v1 roadmap.

## Install

argot is a **single static binary** — no Python, no Node, nothing else to install.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh
```

Prefer a package manager?

```bash
npm install -g @tmonier/argot
```

Both download the prebuilt `argot` binary for your platform (macOS arm64, Linux x64).
Everything runs locally — no API key, no account, nothing leaves your machine.

## Calibrate, then check

argot has three commands. Run them in order the first time, then just `check` on every commit.

```bash
cd your-repo
argot extract      # walk git history → .argot/dataset.jsonl
argot fit          # build the repo corpus + baseline, then calibrate the threshold
argot check        # score uncommitted changes (or pass a ref/range)
```

That's it. `extract` and `fit` are the one-time setup (re-run `fit` after a major refactor); `check`
is the thing you run on every diff.

```text
argot check · 2 hunks above threshold (1 foreign · 1 suspicious)
note: argot is a probabilistic style linter — verify before action.

src/utils/http-helpers.ts
  ●  L42-L48   8.21  foreign     · workdir · foreign import (import)
     ↳ axios — 0 of 47 module specifiers in repo
       common here: react (320×), express (88×), pg (47×)
```

## What argot is — and isn't

- It **does not** replace ESLint, ruff, or your type checker. Those answer *"is this valid?"*
- It **does** answer the second question — *"is this how this team writes things?"* — the one that
  used to live in code review.

If your team ships LLM-assisted code, this is the layer your CI is missing.

## Where to next

- [How it works](/docs/how-it-works/) — the two-phase pipeline, in plain terms.
- [The commands](/docs/the-commands/) — `extract`, `fit`, and `check` in detail.
- [Reading the output](/docs/reading-the-output/) — severity tiers, sources, and the evidence line.
- [What it catches](/docs/what-it-catches/) — three real examples every other tool stays silent on.
