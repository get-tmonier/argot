---
title: Agents (skills & MCP)
description: Put argot in your coding agent's loop — a commit-time check skill and proactive MCP context — advisory, never blocking.
group: Guide
order: 8
---

Most code argot judges is now written by an AI agent. There are two ways to put
argot in that loop, and they compose:

- **Skills** — a commit-time safety net. The agent runs `argot check` on what it
  just wrote and interprets the result.
- **MCP** — proactive context. The agent asks argot for the repo's idioms
  *before* generating, so it writes in-voice from the first token.

Both follow one rule: **argot is advisory and never blocks.** A hit is a prompt
to think, not a gate — the human always has the last word. The full contract
lives in the repo's [`AGENTS.md`](https://github.com/get-tmonier/argot/blob/main/AGENTS.md).

## Skills

Three agent-agnostic skills (Claude Code, Cursor, Codex, …):

| Skill | When it runs |
|---|---|
| `argot-setup` | Once per repo (local) — fit the model and decide what shouldn't shape its voice. |
| `argot-check` | Per change (local) — score the working diff and surface anything foreign. |
| `argot-ci` | Once (CI) — wire the GitHub Action for a non-blocking voice score on every PR. |

In **Claude Code**, install the plugin — it bundles the skills *and* the MCP
server below in one step:

```text
/plugin marketplace add get-tmonier/argot
/plugin install argot@argot
```

For any other agent, use the open [`skills`](https://github.com/vercel-labs/skills)
CLI:

```sh
npx skills add get-tmonier/argot
```

`argot-check` reads `argot check --format json` and follows an advisory decision
tree: it surfaces `foreign` hits prominently, mentions `suspicious` ones, and
stays quiet on `unusual` — and it **never** blocks a commit, rewrites your code,
or mutes on your behalf. When a divergence is intentional, it offers the exact
command to record it:

```text
argot mute <hash> --reason "adopting axios repo-wide"
```

### Drop `AGENTS.md` in your repo

The [AGENTS.md](https://agentsmd.io) standard is read natively by Claude Code,
Codex, Cursor, Aider, Copilot, and more. Copy argot's
[`AGENTS.md`](https://github.com/get-tmonier/argot/blob/main/AGENTS.md) contract
into your repo so any agent — skill installed or not — knows to treat argot as
advice, read the output correctly, and mute false positives with a reason.

## MCP — proactive voice

`argot mcp` runs a [Model Context Protocol](https://modelcontextprotocol.io)
server over stdio, in-process against the fitted `.argot/` model — no network,
no separate runtime. It exposes four tools:

| Tool | When the agent calls it | Returns |
|---|---|---|
| `argot.voice_context` | **before** generating code for a file | typical callees and familiar imports for the file's language — bias generation toward local idioms |
| `argot.check` | on a generated hunk | whether it's out of voice, the score, the reason, and evidence |
| `argot.explain` | to understand a hit | the reason plus the full evidence trail |
| `argot.fit_status` | to gauge trust | corpus composition, calibration freshness, and a Ready / Marginal / Not-recommended verdict |

The point is **writing in-voice from the first token** instead of
writing-then-fixing. Fit the repo first (`argot init`), then wire it up:

```sh
# Claude Code
claude mcp add argot -- argot mcp --repo /path/to/your/repo
```

```json
// .mcp.json / ~/.claude.json, or Cursor's ~/.cursor/mcp.json
{
  "mcpServers": {
    "argot": { "command": "argot", "args": ["mcp", "--repo", "."] }
  }
}
```

Any MCP client works — the server speaks newline-delimited JSON-RPC 2.0 on
stdio. It's local-only by default: it reads `.argot/` on disk and never opens a
socket. The statistics it surfaces are derived from your own repository.

## Which to use

- **Just want a safety net?** Install the skills — zero config, runs at
  commit time.
- **Want the agent to write in-voice up front?** Add the MCP server.
- **Both** is the best of it: proactive context while generating, plus the
  commit-time check. They don't conflict.
