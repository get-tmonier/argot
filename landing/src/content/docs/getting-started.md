---
title: Getting started
description: Install argot and calibrate it on your repo in a couple of minutes.
group: Start
order: 1
---

**argot** learns your repo's patterns from its own git history, then flags AI-written code that
doesn't fit — on five axes: a dependency, API, or construct it has never used (**foreign**); a new
function that reinvents one you already wrote (**redundant**); the right code filed in the wrong
place (**misplaced**); an internal import that reverses your module layering (**layering**); and a
test weakened, disabled, or deleted alongside the production change it covers (**integrity**). All
valid, typed, and lint-clean — but not how anything here is actually built. The base guardrail is
model-free; the semantic layer that finds reinventions and misplacements runs a small local code
embedder — one ~100 MB one-time download, still no cloud, no GPU required, nothing leaves your
machine.

> **Status: alpha.** argot is a probabilistic style linter — treat every flag as a prompt to look,
> and verify before you gate CI on it. It ships honest, leak-free benchmarks and a public research log.

## Install

argot is a **single static binary** — no Python, no Node, no runtime to install. (On first use the
semantic layer fetches a ~100 MB code-embedding model to a local cache — a one-time download, and
still nothing that leaves your machine.)

**macOS / Linux** — package-manager-free installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh
```

**Windows** — the PowerShell equivalent:

```powershell
powershell -c "irm https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.ps1 | iex"
```

**Any platform** — prefer a package manager?

```bash
npm install -g @tmonier/argot
```

All three download the prebuilt `argot` binary for your platform (macOS arm64/Intel, Linux x64/arm64,
Windows x64). Everything runs locally — no API key, no account, nothing leaves your machine.

## Two ways to run it — pick one, or both

- **On your machine** — check as you work (and in a pre-commit hook). Install the
  CLI, `argot init`, then `argot check`. Start below.
- **In CI** — a non-blocking voice score on every pull request. Just add a GitHub
  Actions workflow; **you don't need to set argot up locally for this** — the
  Action installs and fits it for you. See [CI](/docs/ci/).

They're independent: do the local flow, the CI flow, or both. The rest of this
page is the local flow.

## Set up locally, then check

**argot's accuracy is a function of its setup.** It learns from what it's allowed to see — a fit
that ingests vendored SDKs, generated stubs, or data files speaks with the wrong voice and flags
the wrong things. The recommended path is the [setup skill](/docs/setup/) (`npx skills add
get-tmonier/argot`, then `/argot-setup`): your coding agent reads the tree and makes the
what-shapes-the-voice calls for you. Going by hand, make them yourself first:

```bash
cd your-repo
argot init --suggest   # which dirs look like they shouldn't shape the voice
#   → review, add them to argot.toml [exclude].paths, then:
argot init         # learn your repo's voice, then a health check (Ready / Marginal / …)
argot check        # score uncommitted changes (or pass a ref/range)
argot replay       # the fun one: what argot would have caught in your last 50 commits
```

The first `init` also downloads the ~100 MB embedding model to a shared local cache — once per
machine, never per repo. Prefer it explicit? `argot model fetch` pre-downloads it (useful before
going offline or in CI). And `argot rules` lists every rule with its effective severity — all
`error` by default, each one yours to downgrade in `argot.toml`
([Configure](/docs/configure/#rules--rule-severities)).

`argot init` fits the model once and writes a `.argot/.gitignore` so the rebuildable model stays out
of git. If the health check isn't **Ready**, or your repo has generated / vendored / peripheral code
(a monorepo's marketing site, playground, or demo apps), spend a minute on [Setup](/docs/setup/) to
tell argot what shouldn't shape its voice. Then run `check` on every diff.

```text
argot check · 2 hunks above threshold (1 foreign · 1 suspicious)
note: argot is a probabilistic style linter — verify before action.

src/utils/http-helpers.ts
  !  L42-L48   8.21  foreign     · workdir · foreign-import [a1b2c3d4e5f6]
     ↳ axios — 0 of 47 module specifiers in repo
       common here: react (320×), express (88×), pg (47×)
```

## What argot is — and isn't

- It **does not** replace ESLint, ruff, or your type checker. Those answer *"is this valid?"*
- It **reliably** catches what they can't articulate: a **foreign dependency, API, or paradigm** —
  something the repo has never used. When the foreign symbol is in the diff, the base voice model
  catches ~98% of them.
- It **also** flags a **redundant** function (one you already wrote) and **misplaced** code (filed
  in the wrong package) via the semantic layer's per-repo code-embedding index, a **layering**
  break via the architecture graph, and a **test-deleted** / **test-disabled** / **test-weakened**
  hit when a test is removed, skipped, or loosened alongside the production change it exercises —
  each its own rule, each downgradable to `warn` or `off`.
- It is **honest about its limit**: it does *not* reliably flag an *in-vocabulary* choice — a bare
  `ValueError` where you'd raise `HTTPException`, when every token is already yours. So a clean run
  means "no foreign pattern found," **not** "this matches every convention." argot never gates on
  those subtle cases.

If your team ships LLM-assisted code, this is the layer your CI is missing.

> **One setup note:** argot learns from your files *as they are on disk* (anything gitignored is
> skipped automatically), so fit from your **default branch on a clean tree** — uncommitted or
> unmerged foreign code would otherwise be learned as normal. `argot init`/`fit` warns you on a
> dirty tree or a feature branch either way, and the background auto-refresh never makes this
> mistake: it only ever learns accepted history
> ([Health & freshness](/docs/health-and-freshness/)).

## Where to next

- [Setup](/docs/setup/) — configure what argot should (and shouldn't) learn from.
- [Configure](/docs/configure/) — `argot.toml`, inline comments, and durable `argot mute`.
- [How it works](/docs/how-it-works/) — the two-phase pipeline, in plain terms.
- [The commands](/docs/the-commands/) — `init`, `check`, `fit`, `mute`, and the rest in detail.
- [Reading the output](/docs/reading-the-output/) — rules, confidence tiers, sources, and the evidence line.
- [What it catches](/docs/what-it-catches/) — real examples every other tool stays silent on.
