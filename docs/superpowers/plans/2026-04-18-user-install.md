# User Installation End-to-End Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship argot as a self-contained binary installable via curl, npm, or Homebrew, with `argot update` for in-place upgrades and `argot-engine` published to PyPI.

**Architecture:** GitHub Releases is the single artifact store — all install channels (curl, npm, Homebrew) download binaries from it. A lockstep version number (same in `cli/package.json`, `engine/pyproject.toml`, and the git tag) is embedded in the binary at build time. The CLI calls `uvx --from argot-engine==<version>` in production, so the engine version always matches the binary.

**Tech Stack:** Bun compile (`--define`, `--target`), GitHub Actions, GitHub Releases API, PyPI (`uv publish`), Effect CLI, Bash, Node.js postinstall script.

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `cli/src/version.ts` | Create | Single source of the embedded version constant |
| `cli/src/engine-cmd.ts` | Modify | Pin engine version in production mode |
| `cli/src/cli.ts` | Modify | Use `version` from `version.ts`; register `updateCommand` |
| `cli/src/shell/infrastructure/adapters/in/commands/update.command.ts` | Create | `argot update` Effect CLI command |
| `cli/src/shell/infrastructure/adapters/in/commands/update.command.test.ts` | Create | Unit tests for update logic |
| `justfile` | Modify | Add `build` define flag; add `release VERSION` and `publish-engine` recipes |
| `.github/workflows/release.yml` | Create | Tag-triggered release pipeline |
| `install.sh` | Create | curl one-liner install script |
| `npm/package.json` | Create | Thin npm package metadata |
| `npm/scripts/postinstall.js` | Create | Platform binary downloader |
| `npm/bin/.gitkeep` | Create | Placeholder so git tracks the bin directory |
| `README.md` | Modify | Replace install section with three-channel docs |

---

## Task 1: Version constant module

**Files:**
- Create: `cli/src/version.ts`

- [ ] **Step 1: Create `version.ts`**

```typescript
// injected by `bun build --define ARGOT_VERSION=...`; falls back to dev sentinel
declare const ARGOT_VERSION: string;
export const version: string =
  typeof ARGOT_VERSION !== 'undefined' ? ARGOT_VERSION : '0.0.0-dev';
```

- [ ] **Step 2: Update `cli/src/engine-cmd.ts` to accept and pin the version**

Replace the entire file with:

```typescript
import { version } from './version.ts';

export function engineCmd(module: string): { cmd: string; args: string[] } {
  if (process.env['ARGOT_DEV'] === '1') {
    return {
      cmd: 'uv',
      args: ['run', '--package', 'argot-engine', 'python', '-m', module],
    };
  }
  return {
    cmd: 'uvx',
    args: ['--from', `argot-engine==${version}`, 'python', '-m', module],
  };
}
```

- [ ] **Step 3: Update `cli/src/cli.ts` to use the version constant**

Replace line 14:
```typescript
const program = Command.run(app, { version: '0.0.1' });
```
with:
```typescript
import { version } from './version.ts';
// ...
const program = Command.run(app, { version });
```

Full updated `cli/src/cli.ts`:
```typescript
import { Command } from 'effect/unstable/cli';
import { BunRuntime, BunServices } from '@effect/platform-bun';
import { Console, Effect } from 'effect';
import { extractCommand } from '#shell/infrastructure/adapters/in/commands/extract.command.ts';
import { trainCommand } from '#shell/infrastructure/adapters/in/commands/train.command.ts';
import { checkCommand } from '#shell/infrastructure/adapters/in/commands/check.command.ts';
import { explainCommand } from '#shell/infrastructure/adapters/in/commands/explain.command.ts';
import { updateCommand } from '#shell/infrastructure/adapters/in/commands/update.command.ts';
import { AppLive } from '#dependencies';
import { version } from './version.ts';

const app = Command.make('argot', {}, () => Console.log('argot — run `argot --help`')).pipe(
  Command.withSubcommands([extractCommand, trainCommand, checkCommand, explainCommand, updateCommand]),
);

const program = Command.run(app, { version });
program.pipe(Effect.provide(AppLive), Effect.provide(BunServices.layer), BunRuntime.runMain);
```

- [ ] **Step 4: Update `justfile` build recipe to embed the version**

Replace:
```
build:
    mkdir -p dist
    cd cli && bun build --compile --target=bun src/cli.ts --outfile ../dist/argot
```
with:
```
VERSION := `node -p "require('./cli/package.json').version"`

build:
    mkdir -p dist
    cd cli && bun build --compile --target=bun src/cli.ts \
        --define "ARGOT_VERSION=\"$(VERSION)\"" \
        --outfile ../dist/argot
```

- [ ] **Step 5: Run typecheck to verify no type errors**

```bash
just typecheck
```
Expected: passes with no errors.

- [ ] **Step 6: Commit**

```bash
git add cli/src/version.ts cli/src/engine-cmd.ts cli/src/cli.ts justfile
git commit -m "feat: embed version in binary, pin engine version in production"
```

---

## Task 2: `argot update` command

**Files:**
- Create: `cli/src/shell/infrastructure/adapters/in/commands/update.command.ts`
- Create: `cli/src/shell/infrastructure/adapters/in/commands/update.command.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `cli/src/shell/infrastructure/adapters/in/commands/update.command.test.ts`:

```typescript
import { describe, expect, test, mock, beforeEach, afterEach } from 'bun:test';
import { detectTarget, compareVersions, buildDownloadUrl } from './update.command.ts';

describe('detectTarget', () => {
  test('returns linux-x64 on linux', () => {
    expect(detectTarget('linux', 'x64')).toBe('linux-x64');
  });

  test('returns darwin-arm64 on apple silicon', () => {
    expect(detectTarget('darwin', 'arm64')).toBe('darwin-arm64');
  });

  test('returns darwin-x64 on intel mac', () => {
    expect(detectTarget('darwin', 'x64')).toBe('darwin-x64');
  });

  test('throws on unsupported platform', () => {
    expect(() => detectTarget('win32', 'x64')).toThrow('Unsupported platform');
  });
});

describe('compareVersions', () => {
  test('returns "up-to-date" when versions match', () => {
    expect(compareVersions('0.1.0', 'v0.1.0')).toBe('up-to-date');
  });

  test('returns "update-available" when remote is newer', () => {
    expect(compareVersions('0.1.0', 'v0.2.0')).toBe('update-available');
  });

  test('returns "up-to-date" when local is newer (dev build)', () => {
    expect(compareVersions('0.2.0', 'v0.1.0')).toBe('up-to-date');
  });
});

describe('buildDownloadUrl', () => {
  test('builds correct URL', () => {
    expect(buildDownloadUrl('0.2.0', 'darwin-arm64')).toBe(
      'https://github.com/tmonier/argot/releases/download/v0.2.0/argot-darwin-arm64',
    );
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bun test --cwd cli src/shell/infrastructure/adapters/in/commands/update.command.test.ts
```
Expected: FAIL — module not found.

- [ ] **Step 3: Implement the update command**

Create `cli/src/shell/infrastructure/adapters/in/commands/update.command.ts`:

```typescript
import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { writeFileSync, renameSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { version } from '../../../../version.ts';

const REPO = 'tmonier/argot';

export function detectTarget(platform: string, arch: string): string {
  if (platform === 'linux' && arch === 'x64') return 'linux-x64';
  if (platform === 'darwin' && arch === 'arm64') return 'darwin-arm64';
  if (platform === 'darwin' && arch === 'x64') return 'darwin-x64';
  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

export function compareVersions(
  local: string,
  remoteTag: string,
): 'up-to-date' | 'update-available' {
  const remote = remoteTag.replace(/^v/, '');
  const toNum = (v: string) =>
    v.split('.').map(Number).reduce((acc, n, i) => acc + n * Math.pow(1000, 2 - i), 0);
  return toNum(remote) > toNum(local) ? 'update-available' : 'up-to-date';
}

export function buildDownloadUrl(remoteVersion: string, target: string): string {
  return `https://github.com/${REPO}/releases/download/v${remoteVersion}/argot-${target}`;
}

const fetchLatestTag = Effect.tryPromise({
  try: async () => {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: 'application/vnd.github+json' },
    });
    if (!res.ok) throw new Error(`GitHub API error: ${res.status}`);
    const data = (await res.json()) as { tag_name: string };
    return data.tag_name;
  },
  catch: (e) => new Error(`Failed to fetch latest release: ${String(e)}`),
});

const downloadBinary = (url: string, destPath: string) =>
  Effect.tryPromise({
    try: async () => {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`Download failed: ${res.status}`);
      const buffer = await res.arrayBuffer();
      const tmpPath = `${destPath}.tmp`;
      mkdirSync(dirname(tmpPath), { recursive: true });
      writeFileSync(tmpPath, Buffer.from(buffer), { mode: 0o755 });
      renameSync(tmpPath, destPath);
    },
    catch: (e) => new Error(`Failed to download binary: ${String(e)}`),
  });

export const updateCommand = Command.make('update', {}, () =>
  Effect.gen(function* () {
    yield* Console.log('Checking for updates…');

    const latestTag = yield* fetchLatestTag;
    const status = compareVersions(version, latestTag);

    if (status === 'up-to-date') {
      yield* Console.log(`Already up to date (v${version})`);
      return;
    }

    const remoteVersion = latestTag.replace(/^v/, '');
    const target = yield* Effect.try({
      try: () => detectTarget(process.platform, process.arch),
      catch: (e) => new Error(String(e)),
    });

    const url = buildDownloadUrl(remoteVersion, target);
    yield* Console.log(`Downloading argot v${remoteVersion}…`);
    yield* downloadBinary(url, process.execPath);
    yield* Console.log(
      `Updated to v${remoteVersion} — changelog: https://github.com/${REPO}/releases/tag/v${remoteVersion}`,
    );
  }).pipe(
    Effect.catchAll((e) => Console.error(`Update failed: ${e instanceof Error ? e.message : String(e)}`)),
  ),
);
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
bun test --cwd cli src/shell/infrastructure/adapters/in/commands/update.command.test.ts
```
Expected: all 6 tests PASS.

- [ ] **Step 5: Run full verify suite**

```bash
just verify
```
Expected: all checks pass.

- [ ] **Step 6: Commit**

```bash
git add cli/src/shell/infrastructure/adapters/in/commands/update.command.ts \
        cli/src/shell/infrastructure/adapters/in/commands/update.command.test.ts
git commit -m "feat: add argot update command"
```

---

## Task 3: `just release` and `publish-engine` recipes

**Files:**
- Modify: `justfile`

- [ ] **Step 1: Add recipes to `justfile`**

Add after the `bump` recipe:

```
# --- release ---

publish-engine:
    cd engine && uv build && uv publish

release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$(git branch --show-current)" != "main" ]; then
        echo "Error: must be on main branch to release" >&2
        exit 1
    fi
    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: working tree is dirty" >&2
        exit 1
    fi
    # Bump versions
    node -e "
        const fs = require('fs');
        const p = JSON.parse(fs.readFileSync('cli/package.json', 'utf8'));
        p.version = '{{VERSION}}';
        fs.writeFileSync('cli/package.json', JSON.stringify(p, null, 2) + '\n');
    "
    sed -i '' 's/^version = .*/version = "{{VERSION}}"/' engine/pyproject.toml
    # Commit, tag, push
    git add cli/package.json engine/pyproject.toml
    git commit -m "chore: release v{{VERSION}}"
    git tag "v{{VERSION}}"
    git push origin main "v{{VERSION}}"
    echo "Released v{{VERSION}} — CI will build binaries and publish to PyPI"
```

- [ ] **Step 2: Test the recipe dry-run (without pushing)**

```bash
just --dry-run release 0.1.0
```
Expected: prints the commands without executing them, no errors.

- [ ] **Step 3: Commit**

```bash
git add justfile
git commit -m "feat: add just release and publish-engine recipes"
```

---

## Task 4: GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create the workflow**

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

jobs:
  build-binaries:
    if: github.ref_type == 'tag' && startsWith(github.ref, 'refs/tags/v')
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: linux-x64
            os: ubuntu-latest
          - target: darwin-x64
            os: macos-13
          - target: darwin-arm64
            os: macos-latest
    steps:
      - uses: actions/checkout@v4

      - uses: jdx/mise-action@v2
        with:
          install: true

      - name: Install JS dependencies
        run: bun install

      - name: Build binary
        run: |
          VERSION="${{ github.ref_name }}"
          VERSION="${VERSION#v}"
          mkdir -p dist
          cd cli && bun build --compile \
            --target=bun-${{ matrix.target }} \
            --define "ARGOT_VERSION=\"$VERSION\"" \
            src/cli.ts \
            --outfile ../dist/argot-${{ matrix.target }}

      - name: Upload binary artifact
        uses: actions/upload-artifact@v4
        with:
          name: argot-${{ matrix.target }}
          path: dist/argot-${{ matrix.target }}

  publish-engine:
    if: github.ref_type == 'tag' && startsWith(github.ref, 'refs/tags/v')
    name: Publish argot-engine to PyPI
    runs-on: ubuntu-latest
    environment: pypi
    steps:
      - uses: actions/checkout@v4

      - uses: jdx/mise-action@v2
        with:
          install: true

      - name: Publish
        run: cd engine && uv build && uv publish
        env:
          UV_PUBLISH_TOKEN: ${{ secrets.PYPI_TOKEN }}

  create-release:
    name: Create GitHub Release
    needs: [build-binaries]
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - name: Download all binaries
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Create release and attach binaries
        run: |
          gh release create "${{ github.ref_name }}" dist/argot-* \
            --title "argot ${{ github.ref_name }}" \
            --generate-notes
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  update-homebrew-tap:
    name: Update Homebrew tap
    needs: [create-release]
    runs-on: ubuntu-latest
    steps:
      - name: Download darwin binaries for checksums
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Compute SHA256 and update formula
        run: |
          VERSION="${{ github.ref_name }}"
          VERSION="${VERSION#v}"
          SHA_X64=$(sha256sum dist/argot-darwin-x64 | awk '{print $1}')
          SHA_ARM64=$(sha256sum dist/argot-darwin-arm64 | awk '{print $1}')

          # Fetch current formula
          CURRENT=$(gh api repos/tmonier/homebrew-argot/contents/Formula/argot.rb)
          SHA_FILE=$(echo "$CURRENT" | jq -r '.sha')
          CONTENT=$(echo "$CURRENT" | jq -r '.content' | base64 -d)

          # Update urls and checksums
          NEW_CONTENT=$(echo "$CONTENT" \
            | sed "s|releases/download/v[^/]*/argot-darwin-x64|releases/download/v${VERSION}/argot-darwin-x64|g" \
            | sed "s|sha256 \"[a-f0-9]*\" # x64|sha256 \"${SHA_X64}\" # x64|" \
            | sed "s|releases/download/v[^/]*/argot-darwin-arm64|releases/download/v${VERSION}/argot-darwin-arm64|g" \
            | sed "s|sha256 \"[a-f0-9]*\" # arm64|sha256 \"${SHA_ARM64}\" # arm64|")

          ENCODED=$(echo "$NEW_CONTENT" | base64 -w 0)

          gh api repos/tmonier/homebrew-argot/contents/Formula/argot.rb \
            --method PUT \
            --field message="chore: update argot to v${VERSION}" \
            --field content="$ENCODED" \
            --field sha="$SHA_FILE"
        env:
          GH_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release workflow for binaries, PyPI, and Homebrew tap"
```

---

## Task 5: `install.sh` curl script

**Files:**
- Create: `install.sh`

- [ ] **Step 1: Create the script**

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="tmonier/argot"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="argot"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "${OS}-${ARCH}" in
  linux-x86_64)  TARGET="linux-x64" ;;
  darwin-x86_64) TARGET="darwin-x64" ;;
  darwin-arm64)  TARGET="darwin-arm64" ;;
  *)
    echo "Unsupported platform: ${OS}-${ARCH}" >&2
    exit 1
    ;;
esac

# Fetch latest release tag
echo "Fetching latest argot release…"
TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "$TAG" ]; then
  echo "Failed to fetch latest release tag" >&2
  exit 1
fi

URL="https://github.com/${REPO}/releases/download/${TAG}/argot-${TARGET}"

# Download binary
TMP=$(mktemp)
echo "Downloading argot ${TAG} for ${TARGET}…"
curl -fsSL "$URL" -o "$TMP"
chmod +x "$TMP"

# Install
mkdir -p "$INSTALL_DIR"
mv "$TMP" "${INSTALL_DIR}/${BINARY_NAME}"
echo "Installed argot ${TAG} to ${INSTALL_DIR}/${BINARY_NAME}"

# Check uv
if ! command -v uv &>/dev/null; then
  echo ""
  echo "uv not found — installing (required for the Python engine)…"
  curl -LsSf https://astral.sh/uv/install.sh | sh
  echo "uv installed. You may need to restart your shell."
fi

# PATH reminder
if ! echo "$PATH" | grep -q "${INSTALL_DIR}"; then
  echo ""
  echo "Add ${INSTALL_DIR} to your PATH:"
  echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
fi

echo ""
echo "Getting started:"
echo "  cd your-repo"
echo "  argot extract    # parse git history"
echo "  argot train      # train style model (~2GB download first time)"
echo "  argot check      # detect style anomalies"
echo "  argot explain    # AI analysis (requires 'claude' CLI in PATH)"
```

- [ ] **Step 2: Make the script executable and verify it runs without errors (dry check)**

```bash
chmod +x install.sh
bash -n install.sh
```
Expected: no syntax errors printed.

- [ ] **Step 3: Commit**

```bash
git add install.sh
git commit -m "feat: add curl install script"
```

---

## Task 6: npm package

**Files:**
- Create: `npm/package.json`
- Create: `npm/scripts/postinstall.js`
- Create: `npm/bin/.gitkeep`

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p npm/scripts npm/bin
```

- [ ] **Step 2: Create `npm/package.json`**

```json
{
  "name": "@tmonier/argot",
  "version": "0.1.0",
  "description": "Style linter that learns a repo's voice from git history",
  "bin": {
    "argot": "bin/argot"
  },
  "scripts": {
    "postinstall": "node scripts/postinstall.js"
  },
  "keywords": ["git", "style", "linter", "cli"],
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/tmonier/argot.git"
  }
}
```

- [ ] **Step 3: Create `npm/scripts/postinstall.js`**

```js
#!/usr/bin/env node
'use strict';

const https = require('https');
const fs = require('fs');
const path = require('path');

const REPO = 'tmonier/argot';
const BIN_PATH = path.join(__dirname, '..', 'bin', 'argot');

function detectTarget() {
  const platform = process.platform;
  const arch = process.arch;
  if (platform === 'linux' && arch === 'x64') return 'linux-x64';
  if (platform === 'darwin' && arch === 'arm64') return 'darwin-arm64';
  if (platform === 'darwin' && arch === 'x64') return 'darwin-x64';
  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

function get(url, redirects = 5) {
  return new Promise((resolve, reject) => {
    if (redirects === 0) return reject(new Error('Too many redirects'));
    https.get(url, { headers: { 'User-Agent': 'argot-installer' } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return resolve(get(res.headers.location, redirects - 1));
      }
      if (res.statusCode !== 200) return reject(new Error(`HTTP ${res.statusCode}`));
      const chunks = [];
      res.on('data', (c) => chunks.push(c));
      res.on('end', () => resolve(Buffer.concat(chunks)));
      res.on('error', reject);
    }).on('error', reject);
  });
}

async function main() {
  const target = detectTarget();

  // Fetch latest tag
  const meta = JSON.parse(await get(`https://api.github.com/repos/${REPO}/releases/latest`));
  const tag = meta.tag_name;
  const version = tag.replace(/^v/, '');

  const url = `https://github.com/${REPO}/releases/download/${tag}/argot-${target}`;
  console.log(`Downloading argot ${version} for ${target}…`);

  const binary = await get(url);
  fs.mkdirSync(path.dirname(BIN_PATH), { recursive: true });
  fs.writeFileSync(BIN_PATH, binary, { mode: 0o755 });
  console.log(`argot ${version} installed.`);
}

main().catch((e) => {
  console.error('argot postinstall failed:', e.message);
  process.exit(1);
});
```

- [ ] **Step 4: Create `npm/bin/.gitkeep`**

```bash
touch npm/bin/.gitkeep
```

- [ ] **Step 5: Add `npm/bin/argot` to `.gitignore`**

Add to `.gitignore`:
```
npm/bin/argot
```

- [ ] **Step 6: Commit**

```bash
git add npm/ .gitignore
git commit -m "feat: add npm package for @tmonier/argot"
```

---

## Task 7: Homebrew formula (tap repo setup)

This task sets up the `tmonier/homebrew-argot` repo with a formula. This is a **separate GitHub repository** — create it at `https://github.com/tmonier/homebrew-argot` first, then add the formula file there.

- [ ] **Step 1: Create the formula file**

In the `tmonier/homebrew-argot` repo, create `Formula/argot.rb`:

```ruby
class Argot < Formula
  desc "Style linter that learns a repo's voice from git history"
  homepage "https://github.com/tmonier/argot"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/tmonier/argot/releases/download/v#{version}/argot-darwin-arm64"
      sha256 "PLACEHOLDER_ARM64" # arm64
    else
      url "https://github.com/tmonier/argot/releases/download/v#{version}/argot-darwin-x64"
      sha256 "PLACEHOLDER_X64" # x64
    end
  end

  def install
    bin.install stable.url.split("/").last => "argot"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/argot --version")
  end
end
```

- [ ] **Step 2: Add a note to the main repo's `CLAUDE.md` about the tap repo**

Add at the bottom of `CLAUDE.md`:

```
## Homebrew tap

Formula lives in a separate repo: https://github.com/tmonier/homebrew-argot
The `update-homebrew-tap` job in `.github/workflows/release.yml` auto-updates it on each release.
Requires secret `HOMEBREW_TAP_TOKEN` (PAT with `repo` scope on `tmonier/homebrew-argot`) in the main repo's GitHub Actions secrets.
```

- [ ] **Step 3: Commit the CLAUDE.md change**

```bash
git add CLAUDE.md
git commit -m "docs: add note about homebrew-argot tap repo"
```

---

## Task 8: README install section

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Replace the install section in `README.md`**

Find the existing install/setup section and replace it with:

```markdown
## Installation

### curl (recommended)

```sh
curl -fsSL https://raw.githubusercontent.com/tmonier/argot/main/install.sh | sh
```

Installs the `argot` binary to `~/.local/bin` and installs `uv` if missing.

### npm

```sh
npm install -g @tmonier/argot
```

### Homebrew

```sh
brew install tmonier/argot/argot
```

### Prerequisites

| Dependency | Required for | Install |
|---|---|---|
| `uv` | All commands (Python engine) | Installed automatically by curl script, or `curl -LsSf https://astral.sh/uv/install.sh \| sh` |
| `claude` CLI | `argot explain` only | [Claude Code](https://claude.ai/code) |

### Getting started

```sh
cd your-repo
argot extract    # parse git history → .argot/dataset.jsonl
argot train      # train JEPA model → .argot/model.pkl (downloads ~2GB torch once)
argot check      # detect style anomalies in recent commits
argot explain    # AI analysis of flagged hunks (requires claude CLI)
```

### Updating

```sh
argot update
```

### Development setup

```sh
git clone https://github.com/tmonier/argot
cd argot
just install     # bun install + uv sync
just verify      # full check suite
```
```

- [ ] **Step 2: Run verify to confirm nothing broke**

```bash
just verify
```
Expected: all checks pass.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add installation section with three install channels"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Covered by |
|---|---|
| Publish argot-engine to PyPI | Task 3 (`publish-engine` recipe) + Task 4 (CI job) |
| GitHub release with binaries for 3 platforms | Task 4 (`build-binaries` matrix) |
| Install script (curl) | Task 5 |
| npm channel | Task 6 |
| Homebrew channel | Task 7 |
| `argot update` in-place self-update | Task 2 |
| Version lockstep CLI + engine | Task 1 |
| Engine version pinned in production uvx call | Task 1 (`engine-cmd.ts`) |
| uv auto-install in curl script | Task 5 |
| README install docs | Task 8 |
| PYPI_TOKEN + HOMEBREW_TAP_TOKEN secrets | Noted in Task 4 and Task 7 |
| Release only on main | Task 4 (`if` condition on workflow) |

**No gaps found.**

**Secrets needed (manual setup in GitHub repo settings before first release):**
- `PYPI_TOKEN` — PyPI API token for publishing `argot-engine`
- `HOMEBREW_TAP_TOKEN` — GitHub PAT with `repo` scope on `tmonier/homebrew-argot`
