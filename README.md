<p align="center">
  <img src="docs/argot-logo.svg" alt="argot" width="200" />
</p>

<p align="center">
  <strong>Like ESLint, but for the unwritten rules.</strong><br/>
  <em>A two-stage scorer learns your repo's import patterns and token distribution — argot flags what diverges.</em>
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

### Getting started

```sh
cd your-repo
argot extract      # parse git history → .argot/dataset.jsonl
argot train        # collect model-A source files and BPE reference → .argot/model_a.txt, .argot/model_b.json
argot calibrate    # sample calibration hunks, set threshold → .argot/scorer-config.json
argot check        # score uncommitted changes (or pass a ref/range)
```

> **Migrating from a JEPA-based `.argot/` directory?** Delete `.argot/` and re-run the pipeline above — the artifact layout has changed.

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

argot has four commands. Run them in order the first time, then just `check` on every commit.

### 1. Extract

Walks the repo's git history and writes a training dataset:

```bash
argot extract                        # extracts from current directory
argot extract /path/to/other/repo    # or any other repo
```

Output: `.argot/dataset.jsonl` — one record per hunk, with tokenized context and content.

### 2. Train

Collects the repo's source files as model A and copies the generic BPE reference as model B:

```bash
argot train
```

Output: `.argot/model_a.txt` (list of source file paths) and `.argot/model_b.json` (generic token reference). Only needs to be re-run when the codebase changes significantly.

### 3. Calibrate

Samples representative hunks from the repo to determine the scoring threshold, then writes the scorer config:

```bash
argot calibrate                      # samples 500 hunks (default), seed 0
argot calibrate --n-cal 200          # fewer calibration hunks
argot calibrate --repo /path/to/repo
```

Output: `.argot/scorer-config.json` with the BPE threshold for the repo. Re-run after major refactors.

### 4. Check

Scores every hunk in a git ref against the trained scorer and prints a ranked table. Exits non-zero if any hunk is above the threshold.

```bash
argot check                          # check uncommitted changes (default)
argot check HEAD                     # check the last commit
argot check HEAD~5..HEAD             # check a range of commits
argot check --repo /path/to/repo HEAD~5..HEAD
```

```
 SURPRISE  TAG         FILE                          LINE  REF
   1.1642  foreign     source/utils/http_helpers.ts     1  3d5cd8b6
   0.7231  suspicious  source/api/router.ts            42  3d5cd8b6
   0.5800  unusual     source/db/queries.ts            18  3d5cd8b6
```

**Understanding the score**

The surprise score is the BPE log-likelihood ratio for that hunk — how different its token distribution is from the repo's own corpus. A low score means the hunk matches the repo's patterns; higher values mean it diverges.

| Tag | Score range | Meaning |
|---|---|---|
| `ok` | ≤ threshold | Fits the repo's style |
| `unusual` | threshold – threshold+0.3 | Slightly off; worth a glance |
| `suspicious` | threshold+0.3 – threshold+0.6 | Noticeably diverges; review it |
| `foreign` | > threshold+0.6 | Sharply inconsistent with the codebase |

The threshold is set automatically by `argot calibrate`. Override it with `--threshold`.

## How it works

1. **Extract** — walks `git log`, extracts commit diffs, tokenizes each hunk and its surrounding context using a language-aware tree-sitter tokenizer.

2. **Train** — collects the repo's non-test source files into model A (the repo's own token distribution) and copies the bundled generic BPE reference (model B, a broad open-source corpus baseline).

3. **Calibrate** — samples up to 500 representative top-level functions and classes from the repo, scores them through the full two-stage scorer, and sets the BPE threshold to the max score over those normal hunks. Writes `.argot/scorer-config.json`.

4. **Check** — runs the two-stage scorer on the target diff:

   **Stage 1 — import graph:** for each hunk, extracts its import statements and checks whether any imported module is absent from the repo's own first-party import set. A single foreign import immediately flags the hunk (`reason: "import"`).

   **Stage 2 — BPE log-ratio:** tokenizes the hunk with the UnixCoder BPE tokenizer and computes the max per-token log-likelihood ratio between the generic reference corpus (model B) and the repo's corpus (model A). A token that is common in generic open-source code but rare in this repo inflates the score. Prose lines (comments, docstrings) are blanked before scoring to avoid natural-language noise.

   A hunk is flagged if either stage fires. Both scores are always computed and included in the output for diagnostics.

Language-specific logic (import extraction, prose masking, auto-generated file detection, sampleable-range enumeration) is fully encapsulated in `LanguageAdapter` implementations. Python and TypeScript are supported out of the box.

No training data or model leaves your machine. All stages run entirely locally.

## Limitations

- Needs meaningful history (~200+ commits). Below that the scorer has too little signal.
- Best on codebases with a consistent hand. Highly polyglot repos or repos with many contributors and no enforced style are harder to model.
- Cold start on brand-new files: less context to score against.
- Signal is noisier on very small hunks (< 5 lines).

## Stack

**CLI** TypeScript + Bun · **Engine** Python + tree-sitter + HuggingFace tokenizer (UnixCoder BPE)

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
just train            # collect model-A files and BPE reference
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
│       ├── scoring/      # two-stage scorer
│       │   ├── scorers/  # SequentialImportBpeScorer + ImportGraphScorer
│       │   ├── calibration/  # random hunk sampler + calibrate entry point
│       │   ├── adapters/ # LanguageAdapter protocol + Python/TypeScript impls
│       │   ├── filters/  # auto-generated and data-dominant file detection
│       │   └── parsers/  # tree-sitter parse helpers
│       ├── git_walk.py   # pygit2 repo walker
│       ├── tokenize.py   # tree-sitter tokenizer
│       ├── extract.py    # extract → JSONL
│       ├── train.py      # collect model-A files + copy BPE reference
│       ├── check.py      # two-stage scoring entry point
│       ├── stats.py      # shared statistical helpers
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
