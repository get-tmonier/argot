<p align="center">
  <img src="docs/argot-mark.svg" alt="argot" width="72" />
</p>

<h1 align="center">argot</h1>

<p align="center">
  <strong>Like ESLint, but for the unwritten rules.</strong><br/>
  <em>A local JEPA learns your repo's style — argot flags what diverges.</em>
</p>

<p align="center">
  <a href="https://github.com/get-tmonier/argot/actions/workflows/ci.yml"><img src="https://github.com/get-tmonier/argot/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/get-tmonier/argot/blob/main/LICENSE"><img src="https://img.shields.io/github/license/get-tmonier/argot" alt="License" /></a>
  <img src="https://img.shields.io/badge/bun-1.3.11-F472B6" alt="Bun" />
  <img src="https://img.shields.io/badge/python-3.11-3776AB" alt="Python" />
</p>

<p align="center">
  Style linter that learns your codebase's voice from its own git history.<br/>
  Local-first · Zero cloud · Zero telemetry
</p>

---

## What it catches

It does *not* replace ESLint, ruff, or type checkers. It catches what they can't: things that are *technically fine but socially wrong* for this project.

| Signal | What it means |
|---|---|
| **LLM paste-through** | A block whose style diverges sharply from the surrounding file |
| **Convention drift** | Error handling, logging, or patterns that don't match the repo |
| **Debug leftovers** | `console.log`, `.only`, commented-out code you never normally commit |
| **Stylistic outlier** | New code that's correct, but stylistically foreign to this codebase |

## Quick start

```bash
# install
bun add -g argot               # or: brew install argot (eventually)

# train a model on this repo's history (one-time, ~15–30 min on a consumer GPU)
cd my-project
argot train

# lint your staged changes
git add .
argot check                    # lists the most surprising hunks

# lint a specific range
argot check HEAD~5..HEAD
argot check --files src/foo.ts src/bar.ts
```

The model lives in `.argot/model.ckpt`, gitignored by default.

## Where it runs

- **On demand** — `argot check`, same as `eslint .` or `knip`
- **Pre-commit** — `argot install-hook` wires into `.git/hooks/pre-commit`. Opt-in, not the default.
- **CI** — `argot check ${{ github.event.pull_request.base.sha }}..HEAD`, fails above a threshold
- **Editor** — `--json` output makes it trivial to wrap in a VS Code extension or LSP _(planned)_

## How it works

1. **Train** — walks git log, extracts commit diffs, tokenizes with a language-aware tokenizer, trains a small [JEPA](https://github.com/lucas-maes/le-wm) (~15M params) to predict each hunk's embedding given its context.
2. **Check** — runs the same pipeline on the target diff and scores each hunk by *prediction error*. High surprise = "this doesn't sound like the rest of the repo."
3. **Output** — ranked list of hunks + per-hunk score. Exit code non-zero if any hunk exceeds the configured threshold.

No code ever leaves your machine. The model is specific to your repo and useless to anyone else.

## Calibration

Surprise is relative. `argot` uses a rolling percentile calibration: a hunk is flagged only if it's in the top X% of surprising hunks among recent commits. Tune with:

```bash
argot config set threshold 95
```

## Limitations

- Needs meaningful history (~200+ commits). Below that, `argot` will warn and abstain.
- Best on codebases with a consistent hand. Highly polyglot repos with ten different styles are harder to model.
- Cold start on brand-new files: less context to rely on.
- Signal is noisier on very small hunks (< 5 lines).

## Stack

**CLI** TypeScript + Bun · **Training** Python + PyTorch (JEPA adapted from [LeWM](https://github.com/lucas-maes/le-wm)) · **Tokenizer** tree-sitter

---

## Development

### Prerequisites

Install [mise](https://mise.jdx.dev/) then provision the toolchain:

```bash
mise install     # bun 1.3.11 · node 22 · python 3.11 · uv 0.5.0 · just 1.34.0 · lefthook 1.7.0
```

### Setup

```bash
just install          # bun install + uv sync
lefthook install      # wire pre-commit hooks
```

### Tasks

```bash
just verify           # lint + format + typecheck + boundaries + knip + test
just test             # bun test (cli) + pytest (engine)
just extract .        # run the extract pipeline on this repo
just verify-fix       # same as verify but auto-fixes lint/format
```

### Repository layout

```
argot/
├── cli/              # TypeScript CLI (Bun runtime)
│   └── src/
│       ├── cli.ts                    # entrypoint, Effect CLI wiring
│       ├── dependencies.ts           # root Effect Layer composition
│       ├── modules/<name>/           # vertical slice per feature
│       │   ├── domain/               # pure types, no deps
│       │   ├── application/          # use-cases + port interfaces
│       │   └── infrastructure/       # adapters implementing ports
│       └── shell/                    # CLI commands (inbound adapters)
├── engine/           # Python data pipeline (UV workspace)
│   └── argot/
│       ├── git_walk.py   # pygit2 repo walker
│       ├── tokenize.py   # tree-sitter tokenizer
│       ├── extract.py    # CLI entrypoint → JSONL writer
│       ├── train.py      # JEPA training entry point
│       ├── check.py      # scoring entry point
│       └── dataset.py    # msgspec record schema
└── justfile          # task runner (canonical interface)
```

### Tooling

| Tool | Role |
|---|---|
| `mise` | Toolchain version manager |
| `just` | Task runner — single source of truth for all dev commands |
| `bun` | JS runtime, package manager, test runner |
| `uv` | Python package manager and virtual env |
| `oxlint` | Fast TypeScript/JS linter |
| `oxfmt` | TypeScript formatter |
| `tsgo` | TypeScript type-checker (native, ~10× faster) |
| `dependency-cruiser` | Enforces hexagonal layer boundaries |
| `knip` | Dead code and unused dependency detection |
| `lefthook` | Git hook runner |
| `ruff` | Python linter + formatter |
| `mypy` | Python type-checker (strict mode) |

## License

MIT
