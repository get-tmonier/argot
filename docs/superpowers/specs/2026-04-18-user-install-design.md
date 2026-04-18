# argot — End-to-End User Installation Design

Date: 2026-04-18

## Overview

Ship argot as a self-contained binary that end users install via curl, npm, or Homebrew. The Python engine (`argot-engine`) is published to PyPI and invoked transparently via `uvx` — users never touch Python or uv directly. A built-in `argot update` command handles upgrades in place.

---

## 1. Versioning

Single version number shared across the entire project:

- `cli/package.json` → `version`
- `engine/pyproject.toml` → `project.version`
- GitHub release tag → `v<version>`

Both files are bumped together via `just release VERSION`. That recipe:
1. Updates `cli/package.json` and `engine/pyproject.toml`
2. Commits with message `chore: release v<VERSION>`
3. Tags the commit `v<VERSION>`
4. Pushes commit + tag to main

The version string is embedded in the binary at build time via `bun build --define`:
```
VERSION="0.1.0"
```

In production mode (`ARGOT_DEV` unset), `BunEngineRunner` calls:
```
uvx --from argot-engine==<embedded-version> argot-<command>
```
This ensures the installed binary always calls the matching engine version. In dev mode (`ARGOT_DEV=1`), the existing `uv run --package argot-engine` path is preserved.

---

## 2. CI Release Workflow

File: `.github/workflows/release.yml`

Triggered on `v*` tags pushed to main only:
```yaml
on:
  push:
    tags:
      - "v*"
jobs:
  build:
    if: github.ref_type == 'tag' && startsWith(github.ref, 'refs/tags/v')
```

### Jobs (run in parallel)

**build-binaries** — matrix over three targets:
| Target | Bun flag |
|---|---|
| `linux-x64` | `--target=bun-linux-x64` |
| `darwin-x64` | `--target=bun-darwin-x64` |
| `darwin-arm64` | `--target=bun-darwin-arm64` |

Each job runs:
```sh
bun build --compile --target=bun-<target> cli/src/cli.ts \
  --define VERSION=${{ github.ref_name }} \
  --outfile dist/argot-<target>
```
Uploads binary as a GitHub Actions artifact.

**publish-engine** — separate job, does not block binary release on failure:
```sh
cd engine && uv build && uv publish
```
Requires `PYPI_TOKEN` secret.

**create-release** — waits for `build-binaries`, downloads all artifacts, creates the GitHub release and attaches binaries:
```sh
gh release create ${{ github.ref_name }} dist/argot-* \
  --title "argot ${{ github.ref_name }}" \
  --generate-notes
```

---

## 3. Install Channels

All three channels point at GitHub release assets as the single source of truth.

### curl/sh
```sh
curl -fsSL https://raw.githubusercontent.com/tmonier/argot/main/install.sh | sh
```

`install.sh` behaviour:
1. Detects OS + arch (`uname -s` / `uname -m`)
2. Fetches latest release tag from `https://api.github.com/repos/tmonier/argot/releases/latest`
3. Downloads `argot-<target>` binary
4. Places binary at `~/.local/bin/argot` (falls back to `/usr/local/bin`)
5. Checks `uv` in PATH — if missing, installs it via `curl https://astral.sh/uv/install.sh | sh`
6. Prints getting-started message

### npm
```sh
npm install -g @tmonier/argot
```

Thin npm package (no JS runtime logic). Contains only:
- `package.json` with `bin` pointing to `bin/argot`
- `scripts/postinstall.js` — detects platform, downloads matching binary from GitHub releases, places it at `bin/argot`

Pattern used by `@biomejs/biome`, `@tailwindcss/oxide`.

### Homebrew
```sh
brew install tmonier/argot/argot
```

Tap repo: `github.com/tmonier/homebrew-argot`  
Formula: `Formula/argot.rb`

The release workflow updates the formula's `url` and `sha256` fields for both darwin targets via a `gh` API call to the homebrew-argot repo after binaries are uploaded. Requires a `HOMEBREW_TAP_TOKEN` secret (PAT with `repo` scope on `tmonier/homebrew-argot`).

---

## 4. `argot update`

New CLI command. Always self-updates in place regardless of install channel.

Steps:
1. GET `https://api.github.com/repos/tmonier/argot/releases/latest` → parse `tag_name`
2. Compare against embedded `VERSION` — if equal, print "Already up to date" and exit
3. Detect current OS/arch
4. Download new binary to a temp file alongside the current binary
5. Atomically replace: `rename(tmp, currentBinaryPath)`
6. Print "Updated to v<new> — changelog: https://github.com/tmonier/argot/releases/tag/v<new>"

Engine update is automatic: the new binary embeds the new version, so the next `argot` invocation calls `uvx --from argot-engine==<new-version>` and uvx resolves the new package from PyPI.

---

## 5. README Install Section

```sh
# curl (recommended)
curl -fsSL https://raw.githubusercontent.com/tmonier/argot/main/install.sh | sh

# npm
npm install -g @tmonier/argot

# Homebrew
brew install tmonier/argot/argot
```

**Prerequisites:**
- `uv` — required for the Python engine. Installed automatically by the curl script. Otherwise: `curl -LsSf https://astral.sh/uv/install.sh | sh`
- `claude` CLI — required only for `argot explain`

**Getting started:**
```sh
cd your-repo
argot extract    # parse git history → .argot/dataset.jsonl
argot train      # train JEPA model → .argot/model.pkl (downloads torch once, ~2GB)
argot check      # detect style anomalies in recent commits
argot explain    # AI analysis of flagged hunks (requires claude CLI)
```

---

## Out of scope

- Windows support
- Pre-trained generic model (each user trains on their own repo)
- Cloud training / hosted model storage
