<p align="center">
  <img src="docs/argot-logo.svg" alt="argot" width="200" />
</p>

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
| **Foreign paradigm** | Class-based OOP dropped into a functional codebase, wrong import style |
| **Stylistic outlier** | New code that's correct, but doesn't sound like anyone on this team wrote it |

## Installation

### curl (recommended)

```sh
curl -fsSL https://raw.githubusercontent.com/get-tmonier/argot/main/install.sh | sh
```

Installs the `argot` binary to `~/.local/bin` and installs `uv` if missing.

### npm

```sh
npm install -g @tmonier/argot
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
argot check      # score uncommitted changes (or pass a ref/range)
argot explain    # AI analysis of flagged hunks (requires claude CLI)
```

### Updating

```sh
argot update
```

### Development setup

```sh
git clone https://github.com/get-tmonier/argot
cd argot
just install     # bun install + uv sync
just verify      # full check suite
```

## Workflow

argot has four commands. You run them in order the first time, then just `check` (and optionally `explain`) on every commit.

### 1. Extract

Walks the repo's git history and writes a training dataset:

```bash
argot extract                        # extracts from current directory
argot extract /path/to/other/repo    # or any other repo
```

Output: `.argot/dataset.jsonl` — one record per hunk, with tokenized context and content.

### 2. Train

Trains a small JEPA model on the extracted dataset:

```bash
argot train
```

Output: `.argot/model.pkl`. Takes a few minutes on CPU. Only needs to be re-run when you want to refresh the model (e.g. after a major refactor).

### 3. Check

Scores every hunk in a git ref against the trained model and prints a ranked table. Exits non-zero if any hunk is above the threshold.

```bash
argot check                          # check uncommitted changes (default)
argot check HEAD                     # check the last commit
argot check HEAD~5..HEAD             # check a range of commits
argot check --repo /path/to/repo HEAD~5..HEAD
argot check --model /path/to/model.pkl HEAD~5..HEAD
```

```
 SURPRISE  TAG         FILE                          LINE  REF
   1.1642  foreign     source/utils/http_helpers.ts     1  3d5cd8b6
   0.7231  suspicious  source/api/router.ts            42  3d5cd8b6
   0.5800  unusual     source/db/queries.ts            18  3d5cd8b6
```

**Understanding the score**

The surprise score is the model's prediction error for that hunk — how poorly it could predict the hunk's style from its surrounding context. A score of `0` means the hunk is completely in line with what the model expects; higher values mean it diverges from the repo's learned patterns.

| Tag | Score range | Meaning |
|---|---|---|
| `ok` | ≤ threshold | Fits the repo's style |
| `unusual` | threshold – threshold+0.3 | Slightly off; worth a glance |
| `suspicious` | threshold+0.3 – threshold+0.6 | Noticeably diverges; review it |
| `foreign` | > threshold+0.6 | Sharply inconsistent with the codebase |

The default threshold is `0.5`. Adjust it with `--threshold`.

### 4. Explain

For flagged hunks, asks Claude to explain *why* they diverge from the codebase's style:

```bash
argot explain HEAD                   # explain anomalies in the last commit
argot explain HEAD~5..HEAD           # explain anomalies across a range
argot explain --repo /path/to/repo HEAD~5..HEAD
argot explain --model /path/to/model.pkl --dataset .argot/dataset.jsonl HEAD~5..HEAD
```

Output: per-hunk natural language analysis with concrete issues:

```
source/utils/http_helpers.ts:1 (p81.6, commit 3d5cd8b6)
  Mixes unrelated concerns by embedding an Express app inside an HTTP queue
  manager class, contrary to the codebase's narrow single-purpose design.
  • Uses snake_case for private fields (_request_queue) while the codebase uses camelCase exclusively
  • Imports express and lodash — dependencies absent from the rest of the codebase
  • bare Function type violates the strict no-any TypeScript convention enforced here
```

#### Why Claude?

argot's JEPA model detects *which* hunks are anomalous — it produces a surprise score based on how poorly it can predict a hunk's embedding from its context. It does not produce text.

`argot explain` takes the flagged hunks, pairs each one with the lowest-surprise examples from the training data (what "normal" looks like for this repo), and passes both to Claude. Claude sees the contrast and can articulate the specific differences. This is the only step that requires a network call, and it's opt-in — `argot check` is entirely local.

## How it works

1. **Extract** — walks `git log`, extracts commit diffs, tokenizes each hunk and its surrounding context using a language-aware tree-sitter tokenizer.

2. **Train** — fits a bag-of-words vectorizer on the corpus, then trains a small JEPA (Joint Embedding Predictive Architecture): an encoder that embeds context and hunks into the same space, and a predictor that tries to predict the hunk embedding from the context embedding. Surprise = prediction error.

3. **Check** — runs the encoder on the target diff, scores each hunk by prediction error, ranks against the distribution of training scores. Flags hunks above the 75th percentile.

4. **Explain** — emits the flagged hunks as JSONL (file, line, surprise score, percentile, raw hunk text, style examples from training), then for each one spawns `claude --print --output-format json --json-schema` with a prompt that includes the hunk and the style examples as context.

No training data or model leaves your machine. The only external call is the `claude` CLI invocation in `explain`, which goes to Anthropic's API through your existing Claude Code session.

## Limitations

- Needs meaningful history (~200+ commits). Below that the model has too little signal.
- Best on codebases with a consistent hand. Highly polyglot repos or repos with many contributors and no enforced style are harder to model.
- Cold start on brand-new files: less context to score against.
- Signal is noisier on very small hunks (< 5 lines).
- The JEPA model is a small POC (~15M params, BoW features). Detection quality will improve with richer embeddings.

## Stack

**CLI** TypeScript + Bun · **Engine** Python + PyTorch (JEPA) + tree-sitter · **Explain** Claude Code CLI

---

## Development

### Prerequisites

Install [mise](https://mise.jdx.dev/) then provision the toolchain:

```bash
mise install     # bun 1.3.12 · python 3.13 · uv 0.11.7 · just 1.49.0 · lefthook 2.1.6
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
just extract .        # extract training data from this repo
just train            # train model on .argot/dataset.jsonl
just check            # score HEAD~1..HEAD
just build            # compile dist/argot standalone binary
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
├── engine/           # Python data pipeline (uv workspace)
│   └── argot/
│       ├── git_walk.py   # pygit2 repo walker
│       ├── tokenize.py   # tree-sitter tokenizer
│       ├── extract.py    # extract → JSONL
│       ├── train.py      # JEPA training
│       ├── check.py      # surprise scoring
│       ├── explain.py    # percentile ranking + style example selection
│       └── dataset.py    # record schema
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
