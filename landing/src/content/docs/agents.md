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

The bundle contains `argot-setup`, `argot-refresh`, `argot-check`, `argot-review-pr`,
`argot-setup-ci`, `argot-write-rule`, and `argot-suggest-rules`. `argot-refresh` re-audits
scope, structural paths, and mutes before a deliberate fit. A user or agent must select a skill; installation
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

The server exposes six consistently named, read-only tools:

| Tool | Use it when | Scope |
| --- | --- | --- |
| `argot.get_fit_status` | Before trusting learned state | Snapshot completeness, fit suitability, compatibility, and adaptive refresh reasons. |
| `argot.get_voice_context` | Before writing a file | Familiar imports, typical callees, and active replacement guidance for that language. |
| `argot.check_hunk` | While drafting an isolated snippet | Fast fitted-voice signal for one supplied hunk; it does not run the other detector groups. |
| `argot.explain_hunk` | After `check_hunk` needs investigation | The same hunk-level voice result with untruncated identifiers and attestation evidence. |
| `argot.check_changeset` | After editing real repository code | The complete configured voice, semantic, architecture, integrity, and custom-rule pipeline over the worktree, index, range, or one commit. |
| `argot.list_conventions` | When exploring how the team organizes code | Learned internal APIs, placement concentrations, and migrations still in progress. |

A fitted repository is required for model-dependent context and checks; the server returns a
readable tool error when that prerequisite is missing. MCP is passive: connecting it does not cause
a check before a commit, at the end of an agent turn, or at acceptance time. Even
`check_changeset` does not update `.argot/last-check.json`.

Fitting is intentionally not an MCP tool. It changes the shared learned baseline and therefore
belongs to the local `argot-setup` or `argot-refresh` workflow, where scope and mutes are reviewed
before the resulting `.argot/` diff is committed.

## Recommended agent loop

1. Set up the repository with [Init and Fit](/docs/init-and-fit/).
2. Optionally call `argot.get_voice_context` before generating a change.
3. Run `argot.check_changeset` or the full [`argot check`](/docs/check/) on the intended changeset.
4. Read the rule evidence; keep the human decision explicit.
5. When fit status recommends maintenance, use `argot-refresh`; never fit in CI.

Copying Argot’s [`AGENTS.md`](https://github.com/get-tmonier/argot/blob/main/AGENTS.md) into a
repository can give agents the same interpretation contract, but it does not install or trigger
Argot.
