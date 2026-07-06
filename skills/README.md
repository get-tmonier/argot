# argot skills

Two skills that let a coding agent (Claude Code, Cursor, Codex, …) use argot
well — set it up, and check changes against the repo's learned voice **without
ever blocking you**.

| Skill | When it runs |
|---|---|
| [`argot-setup`](./argot-setup/SKILL.md) | Once per repo — fit the model and decide what shouldn't shape its voice. |
| [`argot-check`](./argot-check/SKILL.md) | Per change — score your working diff and surface anything foreign (advisory). |

## Install

With the open [`skills`](https://github.com/vercel-labs/skills) installer
(agent-agnostic):

```sh
npx skills add get-tmonier/argot
```

Or copy the skill folders into your agent's skills directory by hand (for Claude
Code that's `.claude/skills/`):

```sh
mkdir -p .claude/skills
cp -R argot-setup argot-check .claude/skills/
```

Both skills call the `argot` CLI, so install that too — see the
[getting-started guide](https://argot.tmonier.com/docs/getting-started/).

## The one rule

argot is a **statistical** guardrail. These skills treat every hit as *advice*,
never a gate: they surface divergences and offer to record deliberate ones
(`argot mute <hash> --reason "…"`), but they never block a commit, fail a task,
or rewrite your code to satisfy the linter. The human has the last word. The
full contract is in the repo's [`AGENTS.md`](../AGENTS.md).

## Prefer proactive guidance?

The [MCP server](https://argot.tmonier.com/docs/mcp/) (`argot mcp`) exposes the
repo's idioms *before* you generate code, so an agent can write in-voice from
the first token instead of writing-then-checking. Skills and MCP compose — use
the skill for the commit-time safety net, MCP for up-front context.
