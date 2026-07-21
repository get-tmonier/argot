---
title: Agents (skills & MCP)
description: Put argot in your coding agent's loop — a commit-time check skill and proactive MCP context — informational, never blocking.
group: Guide
order: 9
---

Most code argot judges is now written by an AI agent, so the natural place for
argot is inside that agent's loop.

- **Skills** — the primary path, and all most repos need. A commit-time safety
  net: the agent runs `argot check` on what it just wrote and interprets the
  result.
- **MCP** — *optional.* A proactive surface that lets the agent ask argot for the
  repo's idioms *before* generating. It mostly earns its keep on a **large repo**
  the agent can't hold in context — argot's statistical summary is a cheap stand-in
  for reading hundreds of files. On a repo the agent already sees whole, the skills
  alone are enough.

Both follow one principle: **in an agent's loop, argot informs and never blocks.**
A hit is a prompt to think, not a gate — the human always has the last word. The
full contract lives in the repo's
[`AGENTS.md`](https://github.com/get-tmonier/argot/blob/main/AGENTS.md).

## Skills

Six agent-agnostic skills (Claude Code, Cursor, Codex, …):

| Skill | When it runs |
|---|---|
| `argot-setup` | Once per repo (local) — fit the model and decide what shouldn't shape its voice. |
| `argot-check` | Per change (local) — score the working diff and surface anything foreign. |
| `argot-review-pr` | On demand (local) — review one PR or diff range against the repo's voice, no checkout. |
| `argot-setup-ci` | Once (CI) — wire the GitHub Action for a non-blocking voice score on every PR. |
| `argot-write-rule` | On demand (local) — codify a repo convention *you state* as a scripted custom rule, fixture-tested before it ever sees a real diff. |
| `argot-suggest-rules` | On demand (local) — surface the conventions argot *discovered* (`argot conventions`: the repo's vocabulary and where each kind of code lives) and codify a chosen one as a rule. |

In **Claude Code**, install the [plugin](/docs/plugin/) — it bundles the skills,
the MCP server below, *and* the pre-write guardrail in one step:

```text
/plugin marketplace add get-tmonier/argot
/plugin install argot@argot
```

For any other agent, use the open [`skills`](https://github.com/vercel-labs/skills)
CLI:

```sh
npx skills add get-tmonier/argot
```

`argot-check` reads `argot check --format json` and grades its response by
confidence tier: it surfaces `foreign`-confidence hits prominently, mentions
`suspicious` ones, and stays quiet on `unusual` — and it **never** blocks a
commit, rewrites your code, or mutes on your behalf. When a divergence is
intentional, it offers the exact command to record it:

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
server over stdio, in-process against the fitted `.argot/` model — local-first,
no separate runtime. It exposes five tools:

| Tool | When the agent calls it | Returns |
|---|---|---|
| `argot.voice_context` | **before** generating code for a file | typical callees and familiar imports for the file's language — bias generation toward local idioms |
| `argot.conventions` | to learn the repo's conventions | the repo's vocabulary (shared internal API, per language) and its placement conventions — where each kind of code lives |
| `argot.check` | on a generated hunk | whether it's out of voice, the score, the rule that fired, and evidence |
| `argot.explain` | to understand a hit | the rule plus the full evidence trail |
| `argot.fit_status` | to gauge trust | corpus composition, calibration freshness, and a Ready / Ready-with-notes / Not-recommended verdict |

**Tool inputs and responses.** `argot.check` and `argot.explain` take `file_path` and `hunk_content`
(both required) plus optional `file_source` (the full file, for better context); they return
`out_of_voice`, `score`, `threshold`, `rule` (one of the built-in scoring rule names — `rule-tampered` and custom rules never surface through a single-hunk MCP check: `foreign-import`,
`unfamiliar-callee`, `rare-tokens`, `convention`, `superseded`, `redundant`, `misplaced`, `layering`,
`test-deleted`, `test-disabled`, `test-weakened`), `model`,
and — on a hit, or always for `explain` — `evidence`. Either also carries an optional `superseded`
array (`{ old, new, evidence }[]`) whenever the hunk reaches for a pattern the repo has mined or
declared as replaced — a migration nudge alongside whatever else the hunk scores. `argot.voice_context` takes `file_path` (required) and optional `top` (default 10), and
returns `typical_callees_by_cluster`, `familiar_imports`, and the resolved `language`, plus — when
the file's language has any migrations in play — optional `superseded` (`{ avoid, use }[]`) and a
`superseded_note`, so an agent hears "the repo is moving away from this" before it writes more of
it. `argot.fit_status` takes no arguments and returns the full `inspect` report (corpus, calibration,
verdict, reasons). `argot.conventions` takes no arguments and returns the convention catalog
(per-language vocabulary + repo-wide placement + any in-progress migrations, mined or declared).
Tool-level failures come back as an `isError` text
result the agent can read, not a protocol error.

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
stdio. It's local at steady state: it reads `.argot/` on disk and serves
statistics derived from your own repository. The one exception is the semantic
layer's **first use**, which fetches the ~100 MB code-embedding model to a local
cache once; after that — and for everything the base voice model does — nothing
touches the network.

## Pre-write guardrail — ask before foreign code (Claude Code)

The MCP server is passive: the agent has to *choose* to call `voice_context`.
The pre-write guardrail makes it active. It's a Claude Code `PreToolUse` hook
that runs `argot hook` on every `Write`/`Edit`: argot scores the code about to
be written and, **only** when it introduces a dependency the repo has never used
(argot's highest-precision signal), returns an `ask` so you confirm before it
lands — naming the foreign dependency and what the repo uses instead. Everything
else passes silently; it never auto-blocks, and it's a no-op until the repo is
fitted.

It's **opt-in and a no-op until the repo is fitted**. How it's wired depends on
how you installed argot:

- **With the [plugin](/docs/plugin/)** — it's already included. The plugin ships
  the guardrail; once you fit a repo, it starts asking. Nothing to add.
- **With the skills alone** (`npx skills`) — `argot-setup` can add it to your
  `.claude/settings.json` (or `.claude/settings.local.json` for personal-only):

  ```json
  {
    "hooks": {
      "PreToolUse": [
        {
          "matcher": "Write|Edit|MultiEdit",
          "hooks": [
            { "type": "command", "command": "argot hook --repo \"${CLAUDE_PROJECT_DIR}\"", "timeout": 10000 }
          ]
        }
      ]
    }
  }
  ```

  Don't add this **and** run the plugin — the hook would fire twice. To turn it
  off, remove the entry. (Other agents don't expose an equivalent pre-write hook
  yet — use the MCP `voice_context` tool there.)

## Which to use

- **Just want a safety net?** Install the skills — zero config, runs at
  commit time.
- **Want the agent to write in-voice up front?** Add the MCP server.
- **Want a beat before a *new dependency* lands?** Add the pre-write guardrail
  (Claude Code) — it asks before the agent introduces something foreign.
- **Both** is the best of it: proactive context while generating, plus the
  commit-time check. They don't conflict.
