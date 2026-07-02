# argot MCP server

`argot mcp` runs a [Model Context Protocol](https://modelcontextprotocol.io)
server over stdio, exposing the repo's learned voice to LLM coding agents
(Claude Code, Cursor, Aider, …). It runs in-process against the fitted `.argot/`
model — no network, no separate runtime, the same single binary.

Fit the repo first (`argot fit`); the server scores against that model.

## Tools

| Tool | When the agent calls it | Returns |
|---|---|---|
| `argot.voice_context` | **before** generating code for a file | typical callees per cluster + familiar imports for the file's language — bias generation toward the local idioms |
| `argot.check` | on a generated hunk | whether it's out of voice, the score, the reason, and evidence |
| `argot.explain` | to understand a hit | the reason plus the full evidence trail (surprising tokens with repo attestation counts) |
| `argot.fit_status` | to gauge trust | corpus composition, calibration freshness, and a Ready / Marginal / Not-recommended verdict |

The point is **proactive** guidance: an agent that calls `argot.voice_context`
before writing gets the repo's idioms up front and writes in-voice from the
first token, instead of writing-then-fixing.

## Setup

### Claude Code

```sh
claude mcp add argot -- argot mcp --repo /path/to/your/repo
```

or add to `.mcp.json` / `~/.claude.json`:

```json
{
  "mcpServers": {
    "argot": { "command": "argot", "args": ["mcp", "--repo", "."] }
  }
}
```

### Cursor

Add to `~/.cursor/mcp.json` (or the project's `.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "argot": { "command": "argot", "args": ["mcp", "--repo", "."] }
  }
}
```

### Any MCP client (generic)

The server speaks newline-delimited JSON-RPC 2.0 on stdio. Point any MCP client
at the command `argot mcp --repo <path>`. Verify it by hand:

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | argot mcp --repo .
```

## Privacy

Local-only by default: the server reads `.argot/` on disk and never opens a
network socket. The corpus statistics it surfaces (typical callees, imports) are
derived from your own repository — keep the model artifacts out of untrusted
hands the same way you would the source.
