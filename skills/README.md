# argot skills

Four skills that let a coding agent (Claude Code, Cursor, Codex, …) use argot
well — set it up locally, check changes, review a PR, and wire it into CI.
argot watches for four kinds of problem, each with its own rules: code
**foreign** to the repo's learned voice (`foreign-import`, `unfamiliar-callee`,
`rare-tokens`, `convention`), functions the repo **already has** (`redundant`),
code filed in the **wrong area** (`misplaced`), and imports that **break the
repo's layering** (`layering`). Pick the local path, the CI path, or both.

| Skill | Path | When it runs |
|---|---|---|
| [`argot-setup`](./argot-setup/SKILL.md) | local | Once per repo — fit the model, build the semantic index, and decide what shouldn't shape the repo's voice (writes `argot.toml`). |
| [`argot-check`](./argot-check/SKILL.md) | local | Per change — score your working diff against all four detectors and act on what fires. |
| [`argot-review-pr`](./argot-review-pr/SKILL.md) | local | On demand — review a specific PR (or range) against the repo's local model, no checkout. |
| [`argot-setup-ci`](./argot-setup-ci/SKILL.md) | CI | Wire the GitHub Action — a non-blocking score on every PR (no local setup needed). |

## Install

**Claude Code — the plugin (skills + MCP in one step):**

```text
/plugin marketplace add get-tmonier/argot
/plugin install argot@argot
```

Installs all four skills (as `/argot:argot-setup`, `/argot:argot-check`,
`/argot:argot-review-pr`, `/argot:argot-setup-ci`) and the argot MCP server
together.

**Any agent — the `skills` installer** ([vercel-labs/skills](https://github.com/vercel-labs/skills)):

```sh
npx skills add get-tmonier/argot
```

**By hand** — copy the folders into your agent's skills dir (Claude Code: `.claude/skills/`):

```sh
mkdir -p .claude/skills && cp -R argot-setup argot-check argot-review-pr argot-setup-ci .claude/skills/
```

Every path needs the `argot` CLI installed — see the
[getting-started guide](https://argot.tmonier.com/docs/getting-started/).

## The one rule

argot is a **statistical** guardrail; false positives happen. These skills act
on every hit by its *rule* — reuse the existing function, move the misfiled
code, keep the layering intact, stay in the repo's vocabulary — or record a
deliberate divergence (`argot mute <hash> --reason "…"`). They never mute on
your behalf or rewrite code you wrote without asking. The human has the last
word. The full contract is in the repo's [`AGENTS.md`](../AGENTS.md).

## Prefer proactive guidance?

The [MCP server](https://argot.tmonier.com/docs/agents/) (`argot mcp`) exposes the
repo's idioms *before* you generate code, so an agent can write in-voice from
the first token instead of writing-then-checking. Skills and MCP compose — use
the skill for the commit-time safety net, MCP for up-front context.
