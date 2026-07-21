---
title: Plugin (Claude Code)
description: The argot Claude Code plugin — the six skills, the MCP server, and the pre-write guardrail hook in one install, versioned and auto-updating.
group: Guide
order: 10
---

In **Claude Code**, one install gives you everything argot offers an agent —
the six [skills](/docs/agents/), the [MCP server](/docs/agents/#mcp--proactive-voice),
and the [pre-write guardrail hook](#the-pre-write-guardrail) — bundled,
namespaced, and updated together.

```text
/plugin marketplace add get-tmonier/argot
/plugin install argot@argot
```

The first line registers argot's marketplace; the second installs the plugin.
Restart or run `/reload-plugins` and the skills appear as `/argot:argot-setup`,
`/argot:argot-check`, `/argot:argot-review-pr`, `/argot:argot-setup-ci`,
`/argot:argot-write-rule`, and `/argot:argot-suggest-rules`.

## Prerequisite: the `argot` binary

The plugin wires Claude Code to argot; it does **not** ship the engine. Install
the single static binary first — the plugin's skills and MCP server call it:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh
```

Windows: `powershell -c "irm https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.ps1 | iex"` ·
npm: `npm install -g @tmonier/argot`. Full guide:
[Getting started](/docs/getting-started/). Without it on your `PATH`, the skills
tell you to install it and the guardrail hook silently no-ops.

## What the plugin bundles

| Component | What it is | Where it runs |
|---|---|---|
| **Six skills** | `/argot:argot-setup`, `-check`, `-review-pr`, `-setup-ci`, `-write-rule`, `-suggest-rules` — the commit-time safety net, setup flow, and convention-discovery loop | invoked by you or the agent |
| **MCP server** | `argot mcp` — proactive `voice_context` + `conventions` before generating, `check`/`explain` after | when an MCP client connects |
| **Pre-write hook** | `argot hook` — *asks* before a foreign dependency lands | automatically, in fitted repos only |

The skills and MCP server are documented in depth on the
[Agents](/docs/agents/) page — this page covers what's specific to the plugin.

## The pre-write guardrail

The plugin ships the pre-write guardrail as a `PreToolUse` hook, so you don't
wire anything by hand. It runs `argot hook` on every `Write`/`Edit` and — **only**
when the change introduces a dependency the repo has never used (argot's
highest-precision signal) — returns an `ask` so you confirm before it lands,
naming the foreign dependency and what the repo reaches for instead. Everything
else passes silently.

It is designed to cost nothing where it isn't wanted:

- **Opt-in by fitting.** The hook is a no-op until a repo is fitted
  (`.argot/scorer-config.json` exists). Install the plugin, and the guardrail
  only wakes up in repos you've actually set argot up in.
- **No global tax.** A plugin hook is global by nature — it fires on every edit
  in every project while the plugin is enabled. So the command checks for the
  fitted marker *first*: in any repo that isn't fitted, it's a single filesystem
  check and runs no `argot` process at all.
- **Never blocks.** `argot hook` always exits successfully; the strongest thing
  it does is *ask*. The human keeps the last word.

If you install the argot skills **without** the plugin (see below), the guardrail
isn't included — `argot-setup` can wire the equivalent hook into your repo's
`.claude/settings.json` instead. Don't do both: the plugin already provides the
hook, and a second copy in `settings.json` would run it twice.

## Updates

The plugin's version tracks each argot release, so `/plugin update argot` pulls
new skills, MCP changes, and hook fixes as they ship. Once the plugin is listed
in Claude Code's community directory, updates are pulled automatically.

## Not using Claude Code?

The plugin is Claude-Code-specific. For any other agent (Cursor, Codex, and 70+
more), install the skills with the open
[`skills`](https://github.com/vercel-labs/skills) CLI:

```sh
npx skills add get-tmonier/argot
```

That path installs the skills alone — see [Agents](/docs/agents/) for wiring the
MCP server separately, and drop argot's
[`AGENTS.md`](https://github.com/get-tmonier/argot/blob/main/AGENTS.md) contract
into your repo so any agent reads argot's output correctly.

## Uninstall

```text
/plugin uninstall argot@argot
```

This removes the skills, the MCP server, and the hook together. Your repo's
`.argot/` model is untouched — remove it separately if you're done with argot.
