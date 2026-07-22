# Integration capability matrix

**Issue:** EV-01
**Evidence date:** 2026-07-22
**Repository revision inspected:** `98ef01c33f4193715a43da92794c083005284297`

This is an inventory of released or user-installable paths, not a support
promise. “Automatic” means the host invokes the configured integration without
the agent choosing an Argot command. It does **not** mean “at acceptance time.”

## Evidence method

- Repository sources are linked to the inspected revision. The focused source
  smoke was `cargo test -p argot mcp::tests::tools_list_exposes_every_tool`
  and inspection of the declared manifests below.
- Vendor behavior was checked against current official documentation on the
  evidence date. Host documentation establishes host capability; it does not
  prove Argot packaging works on that host.
- The installed local binary reported `argot 0.2.76`; the plugin manifest at the
  inspected revision declares `0.2.82`. This version mismatch is recorded so
  the result is not presented as a release-install smoke.

## Classification key

| Class | Meaning |
| --- | --- |
| Automatic | Host calls the configured integration at its declared lifecycle event. |
| Passive | A connected service is available, but the agent/client must call it. |
| Invoked | A person or agent runs a command/skill deliberately. |
| User-wired | A user must install/configure the host integration; it then runs at that host event. |
| Not verified | The host may support a relevant mechanism, but this revision carries no Argot package/configuration proving that path. |

## Released and user-installable Argot surfaces

| Surface | Trigger and coverage | Prerequisites | Class | Failure/blocking behavior | Tested version/date | Safe wording |
| --- | --- | --- | --- | --- | --- | --- |
| Claude Code plugin: pre-write hook | Claude Code `PreToolUse` for `Write`, `Edit`, and `MultiEdit`; `argot hook` only asks for a flagged `foreign-import`. It is before a write, not after generation or at acceptance. | Plugin enabled; `argot` on `PATH`; fitted `.argot/scorer-config.json`. | Automatic when installed and fitted. | Wrapper is `… || true`; hook exits success and can emit only `permissionDecision: ask`, never deny. Unsupported/unfitted/malformed input silently allows. | Manifest `0.2.82`; source and unit-test inspection, 2026-07-22. | “The Claude Code plugin can ask before a write introduces a foreign dependency in a fitted repo; it does not automatically run full acceptance-time checks.” |
| Claude Code plugin: MCP | Plugin declares `argot mcp --repo .`; server exposes read-only `check`, `explain`, `voice_context`, `fit_status`, and `conventions` tools over stdio. | Plugin enabled; executable on `PATH`; a fitted model for scoring/context tools. | Passive. | Server reports an error result when a fitted model is required but absent; it does not trigger itself. | Manifest `0.2.82`; focused MCP unit test, 2026-07-22. | “The Claude plugin provides optional, agent-invoked MCP context and checks.” |
| Six bundled skills | `argot-setup`, `argot-check`, `argot-review-pr`, `argot-setup-ci`, `argot-write-rule`, and `argot-suggest-rules` are packaged as instructions. They guide an agent when selected; they do not schedule Argot. | Host/plugin skill support; relevant CLI setup per skill. | Invoked. | A skill cannot itself enforce a run; commands retain their documented exit behavior. | Manifest/source inventory, 2026-07-22. | “The plugin bundles six on-demand skills; use a skill when you or your agent chooses the workflow.” |
| `npx skills` installation | Installs Argot skill files for a compatible agent host; no lifecycle hook or MCP configuration is supplied by this repository’s installer instructions. | Node/npm; compatible skills installer and host; Argot CLI for workflows that execute it. | User-wired / invoked. | Installation does not prove host auto-execution; users or agents must invoke the installed skill. | `skills/README.md` inventory, 2026-07-22. | “Skills can give compatible agents an Argot workflow, but they do not make checks automatic.” |
| MCP server outside Claude plugin | `argot mcp` accepts stdio JSON-RPC and serves tools when a client configures and calls it. | A client MCP configuration; `argot` executable; fitted model for score/context operations. | Passive. | The server runs until stdin closes; model-dependent operations return errors rather than a finding when unfitted. | MCP source and focused unit test, 2026-07-22. | “Any MCP-capable client can be configured to call Argot’s read-only tools; the client decides when.” |
| pre-commit hook | On a Git commit, pre-commit runs `argot check --staged` for matched staged source types. | User adds the hook to `.pre-commit-config.yaml`, runs `pre-commit install`, has `argot` on `PATH`, and has fitted the repo. | User-wired, then automatic at commit. | Argot exit `1` rejects that commit; an unfitted repo returns exit `2`. It is not an acceptance-time check and may be bypassed by normal Git/pre-commit controls. | Manifest inspection, 2026-07-22. | “You can wire Argot into pre-commit to check staged code before commits.” |
| GitHub Action | A workflow explicitly using the composite action runs at the workflow’s configured GitHub event and scores the selected ref/range. | User workflow, checkout/history, release-download access and requested permissions. | User-wired, then automatic at workflow event. | Default `fail-on-hits` is `false`; it reports results without gating unless the workflow opts in. | `action.yml` inspection, 2026-07-22. | “The GitHub Action is a workflow-configured, non-blocking PR/push signal by default.” |
| CLI and PR review | A user or agent runs `argot check`, `argot review`, `argot audit`, or related commands. | Installed CLI; a fitted repo where the specific command requires one. | Invoked. | Command-specific exit codes; no host lifecycle invokes these commands by default. | CLI/source inventory, 2026-07-22. | “Run Argot locally or in review when you choose; it is not a default background check.” |

## Repository receipts

| Claim | Repository evidence |
| --- | --- |
| Plugin MCP configuration and declared version | [`plugin.json`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/.claude-plugin/plugin.json) |
| Plugin pre-write trigger, fitted-marker gate, executable gate, and fail-open wrapper | [`hooks/hooks.json`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/hooks/hooks.json) |
| Ask-only, `foreign-import`-only, and silent-allow behavior | [`hook.rs`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/crates/argot-cli/src/hook.rs) |
| MCP transport, tools and fitted-model error behavior | [`mcp.rs`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/crates/argot-cli/src/mcp.rs) |
| Skill inventory and explicit on-demand description | [`skills/README.md`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/skills/README.md) |
| Staged-only pre-commit entry and prerequisites | [`.pre-commit-hooks.yaml`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/.pre-commit-hooks.yaml) |
| Action defaults and `fail-on-hits: false` | [`action.yml`](https://github.com/get-tmonier/argot/blob/98ef01c33f4193715a43da92794c083005284297/action.yml) |

## Current official vendor evidence

| Vendor/source | What it establishes | Consequence for this inventory |
| --- | --- | --- |
| [Anthropic: Claude Code hooks](https://code.claude.com/docs/en/hooks) | Hooks are user-defined handlers at lifecycle events; `PreToolUse` runs before a tool call and can return a decision. Plugin `hooks/hooks.json` is a supported hook location. | The Claude plugin’s pre-write classification is technically feasible, but its precise scope comes from Argot’s own configuration and source. |
| [Anthropic: Claude Code plugins reference](https://code.claude.com/docs/en/plugins-reference) | Plugins can define bundled configuration and MCP servers. | Supports reading the manifest as a Claude plugin integration declaration; it does not establish automatic MCP calls. |
| [pre-commit documentation](https://pre-commit.com/) | `pre-commit install` installs Git hook scripts and then runs configured hooks on commits. | Confirms that the shipped hook is user-wired before it becomes commit-time automatic. |
| [GitHub: workflow triggers](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow) | GitHub Actions workflows run only for configured events. | The composite action cannot run without a user workflow selecting events. |
| [Cursor: Agent overview](https://docs.cursor.com/en/agent/overview) and [tools](https://docs.cursor.com/en/agent/tools) | Cursor Agent can edit/run tools and can use configured MCP servers; its agent behavior is configurable. | Cursor is a potential passive MCP/agent-invoked skills host, but this revision contains no Cursor-specific installation or lifecycle receipt. It is **not verified** as an automatic Argot host. |
| [OpenAI: Codex skills](https://developers.openai.com/codex/skills/) and [Codex plugins](https://developers.openai.com/codex/plugins/) | Codex supports skills/plugins subject to product/workspace availability and permissions. | Codex is a potential skill host, but this revision contains no Codex-specific Argot package or automatic lifecycle receipt. It is **not verified** as an automatic Argot host. |

## Other named agent status

The repository’s historical broad wording (“Cursor, Codex, 70+”) is not an
evidence-backed support matrix. As of this inventory, only Claude Code has an
Argot-bundled automatic lifecycle receipt, and that receipt is intentionally
narrow. Cursor and Codex have documented capabilities relevant to skills/MCP,
but no Argot-specific lifecycle configuration was found at the inspected
revision. Do not claim automatic support for either.

## Reproduction

```sh
git show 98ef01c33f4193715a43da92794c083005284297:hooks/hooks.json
git show 98ef01c33f4193715a43da92794c083005284297:.claude-plugin/plugin.json
cargo test -p argot mcp::tests::tools_list_exposes_every_tool
```

The first command proves the pre-write matcher and fail-open wrapper, the
second proves the declared MCP process, and the test confirms the server’s
advertised tool set. They do not substitute for installing a release artifact
into Claude Code; that is outside EV-01’s repository/manual-smoke scope.
