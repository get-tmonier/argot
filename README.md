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

<!-- TODO(js-lang): JavaScript is landing as a first-class 11th language (own adapter + model). When its
     benchmarked corpus ships, bump "10 languages" here and at the benchmarks section ("27 repos in 10
     languages"), and split the "TypeScript / JS" rows in the benchmark table, Acknowledgements, and the
     tree-sitter grammar list into separate TypeScript and JavaScript entries. Do not change the count until
     the JS numbers land. -->


---

Type checkers and linters answer *"is this valid?"* argot answers the question
that used to live in code review: *"is this how **we** write things here?"* It
learns your codebase's patterns from its git history — no LLM, no GPU, no cloud,
no telemetry — and flags code **foreign to this repo**: a dependency, API, or
whole construct it has never used. Fits in seconds, checks in milliseconds.

If your team ships LLM-assisted code — syntactically perfect, type-correct,
lint-clean, and written in the average voice of every public repo the model
trained on — this is the layer your CI is missing.

## Demo

<p align="center">
  <img src="docs/demo/demo.gif" alt="argot check flagging a foreign Django-style view in an all-FastAPI codebase" width="760" />
</p>

Above: a PR adds a **Django-style class view** to a codebase that is entirely
FastAPI. It's valid Python — mypy and ruff are silent — but the framework it
reaches for is one this repo has never imported. `argot check` groups hits by
file, colors them by severity, and points a `↳` evidence line at the exact token
carrying the score:

```
argot check · 1 hunk above threshold (1 foreign)
note: argot is a probabilistic style linter — verify before action.

fastapi/receipts.py
  !  L1-L10         1.00  foreign  · staged · foreign import (import) [94a92c256ea1]
     ↳ django (L1) — 0 of 74 module specifiers in repo
       common here: fastapi (357×), pydantic (129×), typing (129×) (+7 more)
  1 | from django.views import View
             ^^^^^^
  2 | from django.http import JsonResponse, HttpResponseNotFound
        (+8 more lines)

tip: pass --verbose (-v) to expand truncated hunks.
```

The glyph encodes severity (`!` foreign · `?` suspicious · `.` unusual), the
`[hash]` is a stable id you can `argot mute`, and the `↳` line names the foreign
symbol with the repo's own vocabulary beside it — 74 imports of `fastapi`,
`pydantic`, `starlette`…, and never once `django`. No linter flags a valid import
of a real framework; argot does — because this repo never has.

## Install

argot is a **single static binary** — no Python, no Node, no runtime to install.

```sh
# macOS / Linux (curl)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh

# Windows (PowerShell)
powershell -c "irm https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.ps1 | iex"

# npm (any platform)
npm install -g @tmonier/argot
```

All three download the prebuilt binary for your platform — macOS (Apple Silicon +
Intel), Linux (x64 + arm64), and Windows (x64). See the
[CI guide](https://argot.tmonier.com/docs/ci/) and the
[install docs](https://argot.tmonier.com/docs/) for the full platform matrix.

## Set up

argot is a CLI you point at a repo: `argot init` fits it to your code, `argot check`
scores your changes. Deciding what should — and shouldn't — shape your repo's voice
takes about 30 seconds. Pick a lane.

**Let your coding agent do it — fastest.** Install the skills once:

```sh
npx skills add get-tmonier/argot     # writes SKILL.md files your agent reads
```

Then run **`/argot-setup`** in Claude Code or Cursor (Codex: `$argot-setup`). Your
agent fits the model, decides what to ignore, and verifies the catch — driving the
same `argot` binary you'd run by hand. `/argot-check` scores a diff as you work;
`/argot-ci` wires the GitHub Action. Works across 70+ agents; in Claude Code you can
also `/plugin install argot` to get the skills and the MCP server together.

For proactive context — feeding your repo's idioms to the agent *before* it writes,
not just checking after — add the MCP server:
`claude mcp add argot -- argot mcp --repo .`. Drop argot's [`AGENTS.md`](AGENTS.md)
into your repo so any agent follows the never-block contract, point tools at the
machine-readable [`llms.txt`](https://argot.tmonier.com/llms.txt), and see the
[agents guide](https://argot.tmonier.com/docs/agents/) for the full loop.

**Or run it yourself.**

```sh
cd your-repo
argot init         # learn your repo's voice, then a health check (Ready / Marginal / …)
argot check        # score uncommitted changes (or pass a ref/range)
```

`argot init` fits once and writes a `.argot/.gitignore` so the rebuildable model stays
out of git. Run `check` on every diff — `--staged`, a `HEAD~5..HEAD` range,
`--commit <sha>`, `--min-severity foreign`, or `--format json|sarif` for machines.
`argot update` pulls the latest release.

**Or drive it by hand.** To choose exactly what argot learns from, the
[Setup guide](https://argot.tmonier.com/docs/setup/) walks through `.argotignore` and
a copy-paste prompt for any agent. Full reference: `argot --help` and the
[docs](https://argot.tmonier.com/docs/).

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
can't: code that's **valid, typed, and lint-clean but foreign to this project**.
argot is built for one shape — a **novel pattern** the repo has never used — and
catches **522 of 527 (99%)** when the foreign symbol is visible in the code (the
honest, leak-free bench; see [benchmarks](#benchmarks)). Three shapes it flags,
each a real result from the shipped binary on the FastAPI catalog:

```python
# 1. A foreign dependency — the import stage flags a module the repo never uses.
import requests                    # this codebase standardises on httpx →  ! foreign · requests

# 2. A foreign API — the call-receiver stage flags a call the corpus never attests.
_audit.insert_one({"user": user_id})   # a Mongo call in a SQLAlchemy repo →  ? suspicious · call_receiver

# 3. A foreign paradigm — a whole idiom from another framework.
class ReceiptView(View):           # a Django class-view in an all-FastAPI repo
    def get(self, request, user_id):
        return JsonResponse(...)    # →  ! foreign · call_receiver: JsonResponse, HttpResponseNotFound
```

**The line argot won't cross.** When a break reuses *only* vocabulary the repo
already has — a bare `ValueError` where it usually raises `HTTPException`, a manual
`if status_code >= 400` instead of `raise_for_status()` — every token is
corpus-present and the mistake is a *choice*, not a foreign pattern. argot usually
**does not** flag these and its numbers never gate on them: separating them from
in-voice code drives false alarms (the recovery investigation measured +1 recall
for +45 FP). It catches the danger an agent actually poses — a whole foreign
pattern — not subtle misuse of your own vocabulary. Full, verified breakdown:
[what it catches](https://argot.tmonier.com/docs/what-it-catches/).

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
score on every PR. Copy-paste setups: [the CI guide](https://argot.tmonier.com/docs/ci/),
or run `/argot-ci` (see [Set up](#set-up)).

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

argot's scorer is only as honest as the corpora it's benchmarked against. We use
these repositories as **read-only benchmark corpora** — cloned at benchmark time,
never redistributed — to measure catch and false-alarm rates on real code. None
are affiliated with, endorse, or are endorsed by argot; each remains under its
own license, and their histories are our ground-truth voice signal.

- **Python** — [FastAPI](https://github.com/tiangolo/fastapi) · [rich](https://github.com/Textualize/rich) · [faker](https://github.com/joke2k/faker) · [Saleor](https://github.com/saleor/saleor) · [Wagtail](https://github.com/wagtail/wagtail) · [Dagster](https://github.com/dagster-io/dagster) · [Scrapy](https://github.com/scrapy/scrapy)
- **TypeScript / JS** — [Hono](https://github.com/honojs/hono) · [Ink](https://github.com/vadimdemedes/ink) · [faker-js](https://github.com/faker-js/faker) · [Excalidraw](https://github.com/excalidraw/excalidraw) · [Outline](https://github.com/outline/outline)
- **Go** — [GitHub CLI](https://github.com/cli/cli) · [Hugo](https://github.com/gohugoio/hugo)
- **Rust** — [ripgrep](https://github.com/BurntSushi/ripgrep) · [bat](https://github.com/sharkdp/bat)
- **Java** — [Guava](https://github.com/google/guava) · [JUnit 5](https://github.com/junit-team/junit5)
- **C#** — [PowerShell](https://github.com/PowerShell/PowerShell) · [Jellyfin](https://github.com/jellyfin/jellyfin)
- **C** — [redis](https://github.com/redis/redis) · [curl](https://github.com/curl/curl)
- **C++** — [RocksDB](https://github.com/facebook/rocksdb) · [fmt](https://github.com/fmtlib/fmt)
- **Ruby** — [Homebrew](https://github.com/Homebrew/brew) · [RuboCop](https://github.com/rubocop/rubocop)
- **PHP** — [Laravel](https://github.com/laravel/framework) · [Composer](https://github.com/composer/composer)

Built on [tree-sitter](https://tree-sitter.github.io/tree-sitter/) and its
per-language grammars (Python, TypeScript/JS, Go, Rust, C, C++, Java, C#, PHP,
Ruby), [libgit2](https://libgit2.org/) via [git2](https://docs.rs/git2/) (vendored,
no network transports), HuggingFace
[tokenizers](https://github.com/huggingface/tokenizers) (UnixCoder BPE),
[clap](https://docs.rs/clap/), [Serde](https://serde.rs/), and
[cargo-dist](https://opensource.axo.dev/cargo-dist/) /
[axoupdater](https://github.com/axodotdev/axoupdater) for releases and
`argot update`.

## License

MIT
