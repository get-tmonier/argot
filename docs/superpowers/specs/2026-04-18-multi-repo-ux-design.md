# Multi-repo UX & CLI Verbosity Design

Date: 2026-04-18

## Problem

argot has no concept of repo identity. All paths default to `./argot/` relative to cwd, so:
- Training on one repo then another overwrites nothing (if you're inside each), but there's no way to list, inspect, or manage repos globally
- `--repo` flag doesn't anchor model/dataset paths to that repo's `.argot/` — it only affects the engine, breaking multi-repo usage
- No visibility into what's been extracted/trained for which repo
- CLI is silent during uv package install (appears frozen)
- No per-repo or global preferences (threshold, model)

## Design

### Repo Identity

Repo identity = **absolute git root path** (resolved via `git rev-parse --show-toplevel`, falling back to cwd for non-git directories — future-proofing for codebase audit mode without git).

All commands detect the current repo automatically from cwd. No `--repo` flag.

All `.argot/` paths are resolved from the **git root**, not cwd — so running `argot check` from `src/components/` still finds `.argot/model.pkl` at the repo root.

### Global State: `~/.argot/settings.json`

```json
{
  "version": 1,
  "preferences": {
    "threshold": 0.5,
    "model": "sonnet"
  },
  "repos": {
    "/abs/path/to/bar": {
      "name": "bar",
      "registeredAt": "2026-04-18T09:00:00Z",
      "lastUsedAt": "2026-04-18T09:30:00Z"
    }
  }
}
```

**Auto-registration:** any argot command run inside a new repo registers it automatically — no user action needed. Name defaults to directory basename.

### Local State: `.argot/settings.json` (per-repo, optional)

```json
{
  "preferences": {
    "threshold": 0.3
  }
}
```

Local `preferences` deep-merge over global `preferences`. The `repos` index only ever lives in the global file.

### Command Changes

**Removed flags** from all commands: `--repo`, `--model`, `--dataset`. Paths are always resolved from git root.

| Command | Behaviour |
|---|---|
| `argot extract` | Extracts current repo, writes `<root>/.argot/dataset.jsonl`, auto-registers repo |
| `argot train` | Reads `<root>/.argot/dataset.jsonl`, writes `<root>/.argot/model.pkl` |
| `argot check [ref]` | Reads model from git root; threshold from local → global settings |
| `argot explain <ref>` | Same anchoring; model (LLM) from local → global settings |
| `argot status` _(new)_ | Shows current repo state (see below) |
| `argot list` _(new)_ | Shows all registered repos; works from anywhere |

### New: `argot status`

```
Repo:     bar (~/projects/bar)
Dataset:  1,247 records · 48.2 MB · last extracted 2d ago
Model:    trained 2d ago · 11.8 MB
Settings: threshold 0.5 (global default) · model sonnet
```

### New: `argot list`

```
  NAME       PATH                        DATASET     MODEL
  bar        ~/projects/bar              1,247 rec   trained 2d ago
  foo        ~/projects/foo              —           not trained
* argot      ~/projects/argot            127 rec     trained 5h ago
```

`*` marks the current repo if cwd is inside one.

### CLI Verbosity

**Header line** printed before any operation:

```
argot · bar (~/projects/bar) · 1,247 records · model trained 2d ago
```

**uv/uvx install progress:** pipe stderr from uvx process, detect install lines, show spinner:

```
Installing argot-engine 0.2.6…
```

Spinner disappears once install completes; normal output follows.

## Architecture Impact

### New module: `repo-context`

Responsible for:
- Detecting git root from cwd (`git rev-parse --show-toplevel`)
- Loading and merging global + local settings
- Auto-registering repos in global index
- Exposing resolved paths (`datasetPath`, `modelPath`, `settingsPath`)

This module is a dependency of all command use-cases. It replaces the per-command path arguments.

### Settings port

`SettingsReader` port (application layer) with a `FsSettingsReaderLive` adapter that reads/writes `~/.argot/settings.json` and `.argot/settings.json`.

### Commands updated

All existing commands (`extract`, `train`, `check`, `explain`) lose their path flags and gain a `RepoContext` dependency instead.

`explain` command reads `preferences.model` from merged settings and passes it to the Claude CLI invocation.

## Out of Scope

- `argot repo rename <name>` — can be added later
- Non-git codebase audit mode — identity falls back to cwd path, no other changes needed
- Multiple datasets per repo
