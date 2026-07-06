<p align="center">
  <img src="docs/argot-logo.svg" alt="argot" width="200" />
</p>

<p align="center">
  <strong>A guardrail against code that's foreign to your codebase.</strong><br/>
  <em>argot learns your repo's patterns from its own git history, then flags the dependencies, APIs, and constructs it has never seen — the "unknown to this repo" code an AI coding agent reaches for when it doesn't know your stack.</em>
</p>

<p align="center">
  <a href="https://argot.tmonier.com"><strong>argot.tmonier.com</strong></a>
  &nbsp;·&nbsp;
  <a href="https://argot.tmonier.com/docs/">Documentation</a>
  &nbsp;·&nbsp;
  <a href="docs/research/README.md">Research log</a>
</p>

<p align="center">
  <a href="https://github.com/get-tmonier/argot/releases/latest"><img src="https://img.shields.io/github/v/release/get-tmonier/argot?color=E67E45" alt="Release" /></a>
  <a href="https://www.npmjs.com/package/@tmonier/argot"><img src="https://img.shields.io/npm/v/@tmonier/argot?logo=npm" alt="npm" /></a>
  <a href="https://github.com/get-tmonier/argot/actions/workflows/ci.yml"><img src="https://github.com/get-tmonier/argot/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/get-tmonier/argot/blob/main/LICENSE"><img src="https://img.shields.io/github/license/get-tmonier/argot?color=E67E45" alt="License" /></a>
  <img src="https://img.shields.io/badge/rust-single%20static%20binary-DEA584?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/runtime%20deps-none-brightgreen" alt="No runtime deps" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Python-3776AB?logo=python&logoColor=white" alt="Python" />
  <img src="https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white" alt="TypeScript" />
  <img src="https://img.shields.io/badge/JavaScript-F7DF1E?logo=javascript&logoColor=black" alt="JavaScript" />
  <img src="https://img.shields.io/badge/Go-00ADD8?logo=go&logoColor=white" alt="Go" />
  <img src="https://img.shields.io/badge/Rust-DEA584?logo=rust&logoColor=black" alt="Rust" />
  <img src="https://img.shields.io/badge/Java-ED8B00?logo=openjdk&logoColor=white" alt="Java" />
  <img src="https://img.shields.io/badge/C%23-512BD4?logo=dotnet&logoColor=white" alt="C#" />
  <img src="https://img.shields.io/badge/C++-00599C?logo=cplusplus&logoColor=white" alt="C++" />
  <img src="https://img.shields.io/badge/C-A8B9CC?logo=c&logoColor=black" alt="C" />
  <img src="https://img.shields.io/badge/Ruby-CC342D?logo=ruby&logoColor=white" alt="Ruby" />
  &nbsp;·&nbsp;<a href="#benchmarks">10 languages →</a>
</p>

---

Type checkers and linters answer *"is this valid?"* argot answers the question
that used to live in code review: *"is this how **we** write things here?"* It
builds a statistical model of your codebase's voice from its git history — no
LLM, no GPU, no cloud, no telemetry — and flags hunks whose token shape diverges
from the learned norm. Fits in seconds, checks in milliseconds.

If your team ships LLM-assisted code — syntactically perfect, type-correct,
lint-clean, and written in the average voice of every public repo the model
trained on — this is the layer your CI is missing.

## Demo

<p align="center">
  <img src="docs/demo/demo.gif" alt="argot check flagging an out-of-voice hunk" width="760" />
</p>

`argot check` groups hits by file, colors them by severity, and points a `↳`
evidence line at the exact tokens carrying the score:

```
argot check · 1 hunk above threshold (1 foreign)
note: argot is a probabilistic style linter — verify before action.

fastapi/receipts.py
  !  L1-L16          6.23  foreign  · staged · rare token sequence (bpe) [e500a345aa43]
     ↳ get_receipt (0×), ValueError (17×), resp (0×)
  1 | import httpx
  2 | 
  3 | from fastapi import APIRouter, Depends
  4 | 
  5 | router = APIRouter()
  6 | 
      (+10 more lines)

tip: pass --verbose (-v) to expand truncated hunks.
```

The glyph encodes severity (`!` foreign · `?` suspicious · `.` unusual), the
trailing `[hash]` is a stable id you can `argot mute`, and the `↳` line names
the surprising identifiers with their repo-wide attestation counts —
`ValueError (17×)` appears elsewhere, `resp (0×)` never does; the flag is about
the *combination*, not the words.

## Install

argot is a **single static binary** — no Python, no Node, no runtime to install.

```sh
# curl (recommended)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh

# npm
npm install -g @tmonier/argot
```

Both download the prebuilt binary for your platform — macOS (Apple Silicon +
Intel), Linux (x64 + arm64), and Windows (x64). See the
[CI guide](https://argot.tmonier.com/docs/ci/) and the
[install docs](https://argot.tmonier.com/docs/) for the full platform matrix.

## Quickstart

```sh
cd your-repo
argot init         # learn your repo's voice, then a health check (Ready / Marginal / …)
argot check        # score uncommitted changes (or pass a ref/range)
```

`argot init` fits the model once per repo and writes a `.argot/.gitignore` so the
rebuildable model stays out of git. Run `check` on every diff — `--staged`, a
`HEAD~5..HEAD` range, `--commit <sha>`, `--min-severity foreign`, or
`--format json|sarif` for machines. `argot update` pulls the latest release. Full
reference: `argot --help` and the [docs site](https://argot.tmonier.com/docs/).

## Configuration

argot learns from the code *you* wrote — so it helps to keep generated, vendored,
and data files out of its voice. It already skips tests, docs, examples, build
output, and files it detects as auto-generated or data-only. For the rest, an
`.argotignore` (gitignore-style, layered on the defaults) does the job:

```gitignore
src/generated/        # protobuf / OpenAPI stubs — not our voice
third_party/          # vendored SDKs
```

Don't hand-guess these: `argot init --suggest` finds the generated- and
data-heavy directories for you (with evidence), and a coding agent can name the
vendored or legacy ones from your tree. When a *specific* hit is intentional,
accept it with an audit trail — `argot mute <hash> --reason "…"` — or an inline
`# argot: ignore-next-line`. Full guide:
[Configure](https://argot.tmonier.com/docs/configure/) ·
[Setup](https://argot.tmonier.com/docs/setup/).

## What it catches

It does *not* replace ESLint, ruff, or type checkers — it catches what they
can't: code that's **technically fine but socially wrong** for this project. Each
example below is valid, fully typed, lint-clean, and passes `mypy strict`; every
other tool in your CI is silent on it. (Real fixtures from the FastAPI catalog.
argot's headline catch is a **foreign dependency or API** — 99% when it's visible
in the code, see the [benchmarks](#benchmarks) below; the subtler in-vocabulary
breaks shown here it surfaces without gating on.)

```python
# 1. Wrong exception type — the raise line is the only break.
#    The FastAPI corpus raises HTTPException(status_code=...), never bare ValueError.
raise ValueError(f"User {user_id} not found")            # propagates as 500, not 404

# 2. Structural shape, not vocabulary — every token exists in the corpus,
#    but it uses response.raise_for_status(), not a manual status branch.
if response.status_code >= 400:
    raise HTTPException(status_code=response.status_code, detail=response.text)

# 3. Wrong concurrency model — sync def + blocking I/O, structurally absent
#    from an async codebase where every endpoint is `async def ... await`.
def list_users() -> list[dict[str, Any]]:
    return httpx.get(f"{UPSTREAM_URL}/v1/users").json()
```

### argot vs. the tools you already run

|  | Type checker | Linter · ESLint/ruff | argot |
|---|:---:|:---:|:---:|
| Catches invalid code | ✅ | ✅ | — |
| Enforces a rule you wrote down | — | ✅ | — |
| Flags what's foreign to *this repo* | ❌ | ❌ | ✅ |
| Learns from your history — no rules to write | ❌ | ❌ | ✅ |

argot is additive: it sits *after* your type checker and linter and catches the
one thing they structurally can't — code that's valid and lint-clean but unlike
anything your team has written.

## Benchmarks

**Honest, leak-free numbers.** argot has one job — flag a pattern **foreign to
the repo** (a dependency, API, or construct the codebase has never used, the kind
of thing an AI agent drags in), so the scorecard is two numbers, measured without
leakage:

- **Visible-foreign catch** — foreign imports and APIs spliced into real host
  files and judged by the real `fit` → `check` pipeline. When the foreign symbol
  is visible in the code, argot catches **522/527 (99%)**.
- **False alarm (over-fire)** — a temporal holdout (fit at an old commit, replay
  only commits the model never saw) counting how often argot fires on the repo's
  *own existing code*. Aggregate **0.23%**, worst corpus **0.98%**. A fire on a
  genuinely *new* dependency in a real commit is a **detection**, not an alarm —
  reported separately, never counted against the tool.

| Language | Corpora | Visible-foreign catch | Worst over-fire |
|---|---|---|---|
| Python | fastapi · rich · faker · saleor · wagtail | 101/103 (98%) | 0.35% |
| TypeScript / JS | hono · ink · faker-js · excalidraw · outline | 97/98 (99%) | 0.11% |
| Go | gh-cli · hugo | 38/38 (100%) | 0.98% |
| Rust | ripgrep · bat | 38/38 (100%) | 0.30% |
| Java | guava · junit5 | 38/38 (100%) | 0.58% |
| C# | powershell · jellyfin | 40/40 (100%) | 0.22% |
| C | redis · curl | 37/38 (97%) | 0.18% |
| C++ | rocksdb · fmt | 39/39 (100%) | 0.36% |
| Ruby | homebrew · rubocop | 38/38 (100%) | 0.63% |
| PHP | laravel · composer | 38/38 (100%) | 0.00% |

Across **27 repos in 10 languages**. Earlier published numbers were measured
train-on-test and were materially optimistic — see
[issue #92](https://github.com/get-tmonier/argot/issues/92) and the
[re-measurement evidence](docs/research/evidence/issue92-honest-rebench.md).

**What the two numbers mean.** A commit that introduces a *genuinely new*
dependency or API (a symbol with zero usage in the repo at fit time) is not an
idiomatic commit — flagging it is argot's job, so those fires are counted as
**detections**, not false alarms. The true false-alarm rate is **over-fire**:
argot firing on the repo's *own existing code*, and every one of the 27 corpora
holds it to ≤ 0.98%. The one class argot *cannot* catch is **masked foreign** —
a foreign symbol whose name collides with one the repo already uses, or a dynamic
`import()` — a documented statistical limit (~23%), since a voice model can't
separate foreign code that looks exactly like yours. The full per-corpus table,
new-file rates, and confidence intervals are on the
[benchmarks page](https://argot.tmonier.com/benchmarks), fed from CI so they
can't drift from what ships.

Mixed-language monorepos calibrate **one threshold per language** and dispatch
each hunk by file extension — no single distribution dominates the others.

**Adding a language is a roadmap item, not an architectural blocker.** The
scoring pipeline is language-agnostic; per-language is just a tree-sitter
adapter. We ship a language only *after* benchmarking it honestly on real
corpora — and we publish the numbers it actually gets. Want a corpus
validated? [Open an issue](https://github.com/get-tmonier/argot/issues/new).

## Running in CI

`argot check` emits `--format json` (stable schema) and `--format sarif`
(SARIF 2.1.0 for GitHub code scanning). A composite GitHub Action ships at the
repo root (`uses: get-tmonier/argot@main`), and `.pre-commit-hooks.yaml`
registers an `argot-check` hook. It's non-blocking by default — a visual voice
score on every PR. Copy-paste setups: [the CI guide](https://argot.tmonier.com/docs/ci/).

For LLM coding agents, install the `argot-setup` / `argot-check` skills
(`npx skills add get-tmonier/argot`) or run the `argot mcp` server for proactive
voice context — see [the agents guide](https://argot.tmonier.com/docs/agents/).
Drop argot's [`AGENTS.md`](AGENTS.md) into your repo so any agent follows the
never-block contract, and point agents at the machine-readable
[`llms.txt`](https://argot.tmonier.com/llms.txt).

## How it works

A three-stage scorer runs on each diff hunk: an **import check** (is any
imported module foreign to this repo?), a **BPE surprise** score (how much more
likely is this hunk's token distribution under a generic open-source baseline
than under your repo?), and a **call-receiver penalty** (does it call things
this kind of file never calls?). The model is two frequency tables plus a
callee-cluster partition — no neural network, learned entirely from your
history. The full scoring math, calibration protocol, and the experiment log
that got here live in [docs/research/](docs/research/README.md).

## Contributing

Issues and PRs welcome — start with [CONTRIBUTING.md](CONTRIBUTING.md) for dev
setup (`rustup` + `just`), the `just verify` gate, and how to propose a new
language or corpus. The [good first issues](https://github.com/get-tmonier/argot/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22)
are a good place to start.

```sh
git clone https://github.com/get-tmonier/argot && cd argot
just build       # cargo build --release -p argot → target/release/argot
just verify      # cargo fmt --check + clippy -D warnings + cargo test
```

## Acknowledgements

argot's scorer is only as honest as the corpora it's benchmarked against —
thanks to the maintainers of FastAPI, rich, faker, hono, ink, faker-js, and
Dagster, whose histories serve as our ground-truth voice signal (none are
affiliated with or endorse argot). Built on
[tree-sitter](https://tree-sitter.github.io/tree-sitter/),
[libgit2](https://libgit2.org/), HuggingFace
[tokenizers](https://github.com/huggingface/tokenizers) (UnixCoder BPE), and
[clap](https://docs.rs/clap/).

## License

MIT
