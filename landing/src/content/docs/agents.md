---
title: Other agents and MCP
description: Use Argot skills and read-only MCP context with a compatible host, without assuming lifecycle automation.
group: Configure
order: 3
---

Argot’s CLI is the complete, explicit changeset check. Agent integrations can help an agent choose
that workflow or request context, but they do not create a universal automatic check.

## Skills: invoked workflows

Install the skill bundle in a compatible host:

```sh
npx skills add get-tmonier/argot
```

The bundle contains `argot-setup`, `argot-check`, `argot-review-pr`, `argot-setup-ci`,
`argot-write-rule`, and `argot-suggest-rules`. A user or agent must select a skill; installation
does not configure a hook or MCP server, and a skill does not schedule Argot commands on its own.
The selected workflow may require the `argot` CLI and a fitted repository.

Claude Code has a packaged plugin path; see [Claude Code](/docs/plugin/). Cursor and Codex can be
compatible skills or MCP hosts, but this repository does not ship or test a Cursor- or
Codex-specific Argot lifecycle configuration. Do not treat either as an automatically checked host.

## MCP: client-invoked context

`argot mcp` is a local stdio Model Context Protocol server. Configure a client that supports MCP,
then let that client decide when to call it:

```json
{
  "mcpServers": {
    "argot": { "command": "argot", "args": ["mcp", "--repo", "."] }
  }
}
```

The server exposes read-only `voice_context`, `conventions`, `check`, `explain`, and `fit_status`
tools. A fitted repository is required for model-dependent context and checks; the server returns a
readable tool error when that prerequisite is missing. MCP is passive: connecting it does not cause
a check before a commit, at the end of an agent turn, or at acceptance time.

## Recommended agent loop

1. Set up the repository with [Init and Fit](/docs/init-and-fit/).
2. Optionally ask MCP for context before generating a change.
3. Run the full [`argot check`](/docs/check/) on the intended changeset.
4. Read the rule evidence; keep the human decision explicit.

Copying Argot’s [`AGENTS.md`](https://github.com/get-tmonier/argot/blob/main/AGENTS.md) into a
repository can give agents the same interpretation contract, but it does not install or trigger
Argot.
