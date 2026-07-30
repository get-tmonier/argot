---
title: Getting started
description: Install argot, audit accepted history, then fit a voice you can use in the daily review loop.
group: Start
order: 1
---

Argot learns patterns from a repository’s Git history and surfaces changes that look foreign to
that repository. It is a probabilistic review aid: a finding is a prompt to inspect evidence, not
an instruction to reject a change.

## Install

argot is a **single static binary** — no Python, no Node, no runtime to install, and nothing to
fetch afterwards. The code-embedding model the semantic layer uses is compiled into the binary, so
the first run works on an air-gapped machine and nothing ever leaves it.

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

## Start with accepted history

Run an audit before configuring anything. It creates a temporary worktree, fits a historical base,
and reports what would have prompted review in the surviving base-to-head changes. It does not
modify your tree and exits 0 when it completes.

```bash
cd your-repo
argot audit
```

Read the result in the [Audit guide](/docs/audit/), then decide whether to set up a local voice.

## Fit, then review a change

`init` is the portable setup command. It writes a commented `argot.toml` when one is absent,
gitignores personal `argot.local.toml`, fits the voice, and prints its health. Create that shared
configuration before reviewing suggestions: `--suggest` only reports evidence and never edits or
fits configuration.

```bash
argot init
argot init --suggest
# Review and edit argot.toml [exclude].paths if appropriate.
argot init
argot check
```

Use a clean default-branch checkout for a manual fit whenever possible. A dirty tree or unmerged
source commits on a feature branch are warned about because manual fitting learns files as they are
on disk. The [Init and Fit guide](/docs/init-and-fit/) explains the artifacts and refresh behavior.

The model to keep in mind is deliberately small:

1. **Learn locally.** `init` builds the repository-specific voice and indexes; nothing is uploaded.
2. **Review and commit the baseline.** Commit `argot.toml` and the generated `.argot/` snapshot so
   every clone sees the same learned repository. The embedding model itself is already in the binary.
3. **Check everywhere.** Local tools and CI read that snapshot. Pull requests are judged against the
   base branch copy, and CI never fits.
4. **Refresh only after material accepted drift.** `status`, checks, MCP, and CI explain when the
   learned source/function/layout surface has moved enough. Run `argot-refresh` locally to review
   corpus paths and mutes before fitting and recommitting; there is no default time or commit cadence.

`check` scores the change you select. Its exit code is command-specific: 0 means no error-severity
findings, 1 means review findings, and 2 is a setup or usage error. Read the [Check guide](/docs/check/)
before configuring any local or CI response to those results.

## Set it up properly, once

Scoping decides everything downstream: argot is only as good as the judgment of which code is your
voice. [Set up with your agent](/docs/setup-prompt/) is one copy-pasteable prompt that walks any
coding agent through it — audit, scope, fit, verify, tune, and wire local and CI together. If your
agent supports skills, `npx skills add get-tmonier/argot` gives you the same flow as `argot-setup`.

## Where to next

- [Set up with your agent](/docs/setup-prompt/) — the guided one-sitting setup.
- [Audit](/docs/audit/) — inspect recent accepted history first.
- [Init and Fit](/docs/init-and-fit/) — configure and maintain a local voice.
- [Check](/docs/check/) — choose a changeset and interpret its output.
- [CI and pre-commit](/docs/ci/) — user-wired automation at commit or workflow time.
