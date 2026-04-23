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
  Style linter that builds a statistical model of your codebase's voice from its own git history,<br/>
  then flags hunks whose token distribution diverges from the learned norm.<br/>
  No GPU · No cloud · No telemetry · Runs in seconds after a one-time calibration
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

### Concrete examples

Validated against the FastAPI corpus (50 real PRs, 1,452 hunks). All three examples below were caught at **95%+ recall** by Stage 2 alone — with no import-level hints.

**Foreign framework routing** (`reason: import`, score 5.77)
```python
# flagged — Flask vocabulary in a FastAPI codebase
from flask import Flask, abort, jsonify, request
app = Flask(__name__)

@app.route("/users", methods=["GET", "POST"])
def users() -> object:
    return jsonify(list(_users.values()))
```
FastAPI routes use `@router.get` / `@router.post` with Pydantic-typed parameters. Flask's `@app.route(methods=[...])` + `request` proxy is a fully foreign vocabulary — Stage 1 catches the `flask` import immediately.

**Sync blocking in an async codebase** (`reason: bpe`, score 7.33)
```python
# flagged — requests.get() blocks the event loop
import requests
from fastapi import FastAPI

@app.get("/proxy")
async def proxy():
    resp = requests.get("https://upstream/api")  # blocks all concurrent requests
    return resp.json()
```
No foreign import — `requests` is a standard library. But its token pattern (`requests.get`, `requests.Session`, `cert=`, `timeout=`) is rare in the FastAPI corpus. Stage 2 catches this with 100% recall across all host PRs.

**Wrong validation style** (`reason: bpe`, score 5.82)
```python
# flagged — manual dict validation in a Pydantic codebase
def create_user(body: dict):
    if "name" not in body or not isinstance(body["name"], str):
        raise ValueError("name required")
    if "email" not in body:
        raise ValueError("email required")
    ...
```
FastAPI codebases use Pydantic models for input validation. Manual `isinstance`/`"key" not in body` guards produce token patterns absent from the training corpus.

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

1. **Extract** — walks `git log`, extracts commit diffs, tokenizes each hunk and its surrounding context using a language-aware [tree-sitter](https://tree-sitter.github.io/tree-sitter/) tokenizer. Tree-sitter is an incremental, error-tolerant parser that works on partial and syntactically invalid fragments (essential for mid-block hunk slices) and provides a single uniform interface for every supported language.

2. **Train** — collects the repo's non-test source files into model A (the repo's own token distribution) and copies the bundled generic BPE reference (model B, a broad open-source corpus baseline).

3. **Calibrate** — samples up to 500 representative top-level functions and classes from the repo, scores them through the full two-stage scorer, and sets the BPE threshold to the max score over those normal hunks. Writes `.argot/scorer-config.json`.

4. **Check** — runs the two-stage scorer on the target diff:

   **Stage 1 — import graph:** for each hunk, extracts its import statements and checks whether any imported module is absent from the repo's own first-party import set. A single foreign import immediately flags the hunk (`reason: "import"`).

   **Stage 2 — BPE log-ratio:** tokenizes the hunk with the [UnixCoder](https://huggingface.co/microsoft/unixcoder-base) BPE tokenizer (pre-trained on 9M+ code files across 9 languages — only the vocabulary is used, not the neural network) and computes a max-surprise score over the hunk's tokens:

   ```
   P_A(t) = count_A(t) / total_A + ε     # repo-specific token frequency
   P_B(t) = count_B(t) / total_B + ε     # generic open-source token frequency

   surprise(t)  = log P_B(t) − log P_A(t)
   score(hunk)  = max  surprise(t)
                   t ∈ tokens(hunk)
   ```

   A high score means at least one token in the hunk is far more common in generic open-source code than in *this* repo — a reliable signal of foreign style. Model A is built by counting BPE tokens across the repo's non-test source files (CPU-only, takes seconds). Model B is a pre-built reference distribution bundled with argot — no download, no training loop. Prose lines (comments, docstrings) are blanked before scoring to avoid natural-language noise inflating the signal.

   A hunk is flagged if either stage fires. Both scores are always computed and included in the output for diagnostics.

Language-specific logic (import extraction, prose masking, auto-generated file detection, sampleable-range enumeration) is fully encapsulated in `LanguageAdapter` implementations. Python and TypeScript are supported out of the box.

No training data or model leaves your machine. All stages run entirely locally.

## Validation

argot's scorer was benchmarked across 5 open-source repos (Python + TypeScript), 172+ real merged PRs, and a handcrafted catalog of 39 paradigm-break fixtures.

### Controlled break-detection experiment

The cleanest signal: calibrate on repo-native hunks, inject paradigm-break fixtures, measure recall and FP rate on a held-out control set. Run with 5 independent random seeds per corpus.

| Corpus | Seeds | Calibration hunks | Recall | FP rate | Threshold CV |
|---|---|---|---|---|---|
| FastAPI | 5 | 100 | **100%** | mean 1% (1 FP at seed 2) | 3.5% — STABLE |
| Rich | 5 | 100 | **100%** | mean 1% (1 FP at seed 2) | 3.8% — STABLE |
| faker (Python) | 1 | 139 | **100%** | 0% | — |

All single FP events were at a margin of < 0.06 above threshold — within calibration noise. Gate criteria: recall ≥ 100%, FP rate ≤ 5%, threshold CV < 5%. **All three corpora passed all three gates.**

### Recall by paradigm-break category

31 categories × 4 host PRs = 124 injected hunk pairs. Scored through Stage 2 only (import detection disabled), to isolate BPE signal.

| Category | Examples | Catch rate |
|---|---|---|
| Framework swap | Flask routing, Django CBV, aiohttp handler, Tornado handler | 100% |
| Async violations | `requests` blocking event loop, sync file I/O in async | 100% |
| Validation style | Manual dict guards, Cerberus, Voluptuous, assert validation | 75–100% |
| Serialization | Manual JSON dict, msgpack, orjson | 100% |
| Exception handling | Bare `except:`, exception swallow, traceback in response | 100% |
| Background tasks | `threading.Queue`, `atexit`, `multiprocessing` | 100% |
| Dependency injection | Manual class instantiation vs `Depends(...)` | 100% |

8 language-idiom fixtures (walrus operator, match/case, dataclasses, f-strings, async adoption, generator expressions, type annotations, union syntax) were caught at **100% recall** on both FastAPI and Rich.

**Overall Stage-2 recall: 95.2%** (118/124 pairs; two fixtures — Voluptuous validation and an imperative route loop — scored below threshold on some hosts with narrow margins).

### Threshold sensitivity (recall–FP operating curve)

Measured on FastAPI across 35 real merged PRs (clean test set, all flags treated as FP):

| Threshold strategy | Recall (catalog breaks) | Hunk flag rate on real PRs |
|---|---|---|
| `max` (production) | 93.5% | 1.5% |
| p99 | 96.8% | 2.3% |
| p95 | 100.0% | 4.4% |
| p90 | 100.0% | 7.1% |

Production uses `max(cal_scores)` — the most conservative choice. Moving to p95 recovers the 6.5% recall gap at a 3× FP-rate cost.

### Base-rate experiment (false-positive rate on real PRs)

Unfiltered flag rate on real merged PRs — the realistic noise floor in production use:

| Corpus | Language | PRs | Hunks | Flag rate | Confirmed FPs |
|---|---|---|---|---|---|
| FastAPI | Python | 50 | 1,452 | 4.0% | 0 |
| Rich | Python | 37 | 194 | 11.0% | 0 (21 auto-gen suppressed) |
| faker (Python) | Python | 50 | 130 | 0.8% | 1 — Stage 1 on a pure string edit in an import-heavy file |
| faker-js | TypeScript | 5 | 46 | 4.3% | 0 — 2 flags on a genuine `LocaleProxy` core refactor |

All flags in FastAPI, Rich, and faker-js were reviewed manually. Rich's 11% flag rate reflects a PR that added machine-generated Unicode data tables — 21 of those hunks were correctly suppressed by the auto-generated file filter; the remaining flags were on hand-written code that legitimately restructured the import graph.

### Known limits

- **Locale-heavy corpora:** The `max(cal_scores)` strategy degenerates when a large fraction of source files are locale/data tables (tested: faker Python at 50 PRs). The calibration ceiling rises above the observable PR score range, making Stage 2 structurally blind. Use p95 threshold for such repos.
- **Stage 1 file-level import contamination:** Stage 1 can fire on unchanged import lines in the surrounding file when the full file source is not passed. The `check` command always passes `file_source`, so this is not observable in normal use.

## Limitations

- Needs meaningful history (~200+ commits). Below that the scorer has too little signal.
- Best on codebases with a consistent hand. Highly polyglot repos or repos with many contributors and no enforced style are harder to model.
- Cold start on brand-new files: less context to score against.
- Signal is noisier on very small hunks (< 5 lines).

## Stack

**CLI** TypeScript + Bun · **Engine** Python + tree-sitter + HuggingFace tokenizer (UnixCoder BPE) · **Model** two frequency tables + max log-ratio — no neural network, no GPU

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
