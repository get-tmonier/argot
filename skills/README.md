# argot skills

Seven skills that let a coding agent (Claude Code, Cursor, Codex, …) use argot
well — set it up locally, maintain its learned snapshot, check changes, review
a PR, wire it into CI, and codify the repo's own conventions as custom rules.
argot watches for five kinds of problem, each with its own rules: code
**foreign** to the repo's learned voice (`foreign-import`, `unfamiliar-callee`,
`rare-tokens`, `convention`), functions the repo **already has** (`redundant`),
code filed in the **wrong area** (`misplaced`), imports that **break the
repo's layering** (`layering`), and tests **weakened, disabled, or deleted**
alongside a production change (`test-weakened`, `test-disabled`,
`test-deleted`) — plus whatever custom rules a repo has written for itself.
Pick the local path, the CI path, or both.

Skills are selected, on-demand workflows: installing them does not schedule an
Argot check. The GitHub Action is separate and runs only after a repository
adds a workflow for a GitHub event.

The shared lifecycle is intentionally small: `argot-setup` learns locally and
prepares a reviewed `argot.toml` + `.argot/` commit; local checks, agents, and CI
read that same baseline; `argot-refresh` revisits scope and mutes only after
material accepted drift. CI never fits, and there is no default time or commit
cadence.

| Skill | Path | When it runs |
|---|---|---|
| [`argot-setup`](./argot-setup/SKILL.md) | local + CI | Once per repo, one sitting — audit the history, decide what shapes the voice, fit, verify the catch, tune the rules the repo's own history says are noisy, and wire where it runs (hook, pre-commit, MCP, CI). Every decision proposed with its measurement. |
| [`argot-refresh`](./argot-refresh/SKILL.md) | local | On recommendation — re-audit corpus scope and structural path changes, review stale mutes with one explicit approval, then fit and verify the committed snapshot. |
| [`argot-check`](./argot-check/SKILL.md) | local | Per change — score your working diff against every configured rule — the six learned detectors plus any custom rules — and act on what fires. |
| [`argot-review-pr`](./argot-review-pr/SKILL.md) | local | On demand — review a specific PR (or range) against the repo's local model, no checkout. |
| [`argot-setup-ci`](./argot-setup-ci/SKILL.md) | CI | Wire the GitHub Action alone — a non-blocking score on every PR. `argot-setup` covers this as one of its phases; come here when CI is all that's wanted. |
| [`argot-write-rule`](./argot-write-rule/SKILL.md) | local | On demand — codify a repo convention as a scripted custom rule, fixture-tested before it ever sees a real diff. |
| [`argot-suggest-rules`](./argot-suggest-rules/SKILL.md) | local | On demand — turn a convention that `argot conventions` discovered into a fixture-tested custom rule. |

## Install

**Claude Code — the plugin (skills + MCP in one step):**

```text
/plugin marketplace add get-tmonier/argot
/plugin install argot@argot
```

Installs all seven skills (as `/argot:argot-setup`, `/argot:argot-refresh`, `/argot:argot-check`,
`/argot:argot-review-pr`, `/argot:argot-setup-ci`, `/argot:argot-write-rule`,
`/argot:argot-suggest-rules`),
the argot MCP server, and a pre-write guardrail hook — together. The hook is
opt-in and non-blocking, and it only ever activates in a repo you've **fitted**
with argot: in any other repo it's a single filesystem check that runs no
`argot` process at all — so the plugin adds no cost to your other projects.
Where it is active, it never blocks a write; at most it **asks** you to confirm
when an edit reaches for a dependency the repo has never used.

**Any agent — the `skills` installer** ([vercel-labs/skills](https://github.com/vercel-labs/skills)):

```sh
npx skills add get-tmonier/argot
```

**By hand** — copy the folders into your agent's skills dir (Claude Code: `.claude/skills/`):

```sh
mkdir -p .claude/skills && cp -R argot-setup argot-refresh argot-check argot-review-pr argot-setup-ci argot-write-rule argot-suggest-rules .claude/skills/
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
the first token instead of writing-then-checking. It also exposes a read-only
`argot.check_changeset` tool for the complete configured detector pipeline; fitting
is intentionally left to the reviewed setup/refresh skills. Skills and MCP
compose — use the skill for the human-guided workflow and MCP for context or
an explicitly requested check.
