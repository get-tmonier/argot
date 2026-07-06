# argot skills

Three skills that let a coding agent (Claude Code, Cursor, Codex, …) use argot
well — set it up locally, check changes, and wire it into CI — all **without
ever blocking you**. Pick the local path, the CI path, or both.

| Skill | Path | When it runs |
|---|---|---|
| [`argot-setup`](./argot-setup/SKILL.md) | local | Once per repo — fit the model and decide what shouldn't shape its voice. |
| [`argot-check`](./argot-check/SKILL.md) | local | Per change — score your working diff and surface anything foreign (advisory). |
| [`argot-ci`](./argot-ci/SKILL.md) | CI | Wire the GitHub Action — a non-blocking voice score on every PR (no local setup needed). |

## Install

**Claude Code — the plugin (skills + MCP in one step):**

```text
/plugin marketplace add get-tmonier/argot
/plugin install argot@argot
```

Installs both skills (as `/argot:argot-setup` and `/argot:argot-check`) and the
argot MCP server together.

**Any agent — the `skills` installer** ([vercel-labs/skills](https://github.com/vercel-labs/skills)):

```sh
npx skills add get-tmonier/argot
```

**By hand** — copy the folders into your agent's skills dir (Claude Code: `.claude/skills/`):

```sh
mkdir -p .claude/skills && cp -R argot-setup argot-check .claude/skills/
```

Every path needs the `argot` CLI installed — see the
[getting-started guide](https://argot.tmonier.com/docs/getting-started/).

## The one rule

argot is a **statistical** guardrail. These skills treat every hit as *advice*,
never a gate: they surface divergences and offer to record deliberate ones
(`argot mute <hash> --reason "…"`), but they never block a commit, fail a task,
or rewrite your code to satisfy the linter. The human has the last word. The
full contract is in the repo's [`AGENTS.md`](../AGENTS.md).

## Prefer proactive guidance?

The [MCP server](https://argot.tmonier.com/docs/agents/) (`argot mcp`) exposes the
repo's idioms *before* you generate code, so an agent can write in-voice from
the first token instead of writing-then-checking. Skills and MCP compose — use
the skill for the commit-time safety net, MCP for up-front context.
