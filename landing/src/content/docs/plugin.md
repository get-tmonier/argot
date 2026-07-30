---
title: Claude Code
description: The Claude Code plugin bundles on-demand skills, passive MCP tools, and an opt-in pre-write ask.
group: Configure
order: 2
---

This is the canonical guide for the Argot Claude Code plugin. It packages seven skills, an MCP
server, and one narrow hook. These are different capabilities:

| Surface | Invocation | Coverage |
| --- | --- | --- |
| Skills | User or agent selects one | Seven on-demand workflows, including guided snapshot refresh; no scheduling by a skill. |
| MCP | Agent selects a tool call | Read-only context and checks over stdio; it does not invoke itself. |
| Pre-write hook | Automatic when plugin is enabled and repository is fitted | Asks only before a `Write`, `Edit`, or `MultiEdit` introduces a foreign import. |

There is no packaged automatic end-of-turn or acceptance-time full-check lifecycle. Run
[`argot check`](/docs/check/) when you want the full changeset scored.

## Install and prerequisites

```text
/plugin marketplace add get-tmonier/argot
/plugin install argot@argot
```

The plugin requires `argot` on `PATH`. Its pre-write hook is a no-op until the repository has a
fitted `.argot/scorer-config.json`; set that up with [Init and Fit](/docs/init-and-fit/). MCP tools
that need the repository model also require a fitted repository.

The plugin declares `argot mcp --repo .`. MCP is passive: Claude Code may call a tool when it
chooses, but the server does not initiate checks. The available tools provide voice context,
conventions, checks, explanations, and fit status.

## The pre-write ask

The hook runs before Claude Code write tools and only asks about an introduced `foreign-import`.
It never denies the write, does not run a full check, and allows malformed, unsupported, or unfitted
cases through. It is an opt-in review beat, not a merge or acceptance gate.

Do not add a second equivalent `PreToolUse` hook to `.claude/settings.json` while this plugin is
enabled: both hooks would run. If you installed skills without the plugin, the setup skill can
provide a repository-local hook configuration instead.

## Update, opt out, and remove

Use `/plugin update argot` to receive a new published plugin version. To opt out of the pre-write
ask while keeping the plugin, disable or remove its hook through your Claude Code plugin settings.
To remove the packaged skills, MCP declaration, and hook together:

```text
/plugin uninstall argot@argot
```

Uninstalling the plugin does not remove the repository’s `.argot/` artifacts or `argot.toml`.
Remove those separately only if you no longer want the repository configured.
