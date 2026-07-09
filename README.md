<p align="center">
  <img src="docs/argot-logo.svg" alt="argot" width="200" />
</p>

<p align="center">
  <strong>Your codebase has a voice. argot makes AI code speak it.</strong><br/>
  <em>A local guardrail that catches AI-written code that doesn't fit your repo — a dependency you've never used, a function you already wrote, logic in the wrong place. Learned from your git history. Backed by a code-embedding model that runs on your laptop — no LLM, no cloud, no GPU.</em>
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
  <img src="https://img.shields.io/badge/100%25-local%20%C2%B7%20no%20cloud-brightgreen" alt="100% local, no cloud" />
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
  &nbsp;·&nbsp;<a href="#benchmarks">11 languages →</a>
</p>

<!-- TODO(js-numbers): the benchmark TABLE below (per-language rows + "N repos") still shows the
     pre-JavaScript run; refresh it — split the TypeScript/JavaScript row and update the counts —
     once the JS re-bench dashboard lands. -->


---

Type checkers ask *"is this valid?"* argot asks the question that used to live in code review: *"is this how **we** do it here?"* — and catches AI code that's flawless, type-correct, lint-clean, and still doesn't belong.

### Three ways AI code fails to fit — invisible to every linter

|  |  |  |
| :-- | :-- | :-- |
| 🚫 **Foreign** | a dependency, API, or idiom your repo has **never used** | *"we don't do it this way here"* |
| ♻️ **Redundant** | a new function that **reinvents one you already have** | *"you already have this"* |
| 📍 **Misplaced** | the right code, filed in the **wrong place** | *"this doesn't belong here"* |

Copilot, ESLint, SAST — every tool judges by one *global* idea of good code. argot learns **yours**, from your git history, and judges each AI diff against it. That per-repo judgment can't be copied by a bigger model — only by knowing your codebase.

### Real semantic understanding — no LLM, no cloud, no GPU

- ⚡ **Rust · single static binary** — fits in seconds, checks a diff in ~150 ms
- 🧠 **A local code-embedding model** (`jina-code`) — semantic understanding from an encoder that turns code into vectors, **not an LLM**: no generation, no API key, no GPU
- 🪶 **~100 MB model, CPU-first** (Metal-accelerated on Macs) — a few hundred MB of RAM, not the gigabytes a served model needs
- 🔒 **Nothing leaves your machine** — no telemetry, no account, local by default
- 📊 **Honest, leak-free benchmarks** — **98%** foreign catch · **0.22%** false alarms · 31 repos · 11 languages

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
of a real framework; argot does — because this repo never has. Full anatomy:
[Reading the output](https://argot.tmonier.com/docs/reading-the-output/).

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

Point argot at a repo and let your coding agent drive it — the fastest path.
Install the skills once:

```sh
npx skills add get-tmonier/argot
```

Then run **`/argot-setup`** in Claude Code, Cursor, or 70+ agents. This is where
the skill earns its keep: it **reads your codebase** to decide what should and
shouldn't shape the repo's voice — a vendored SDK, a generated `gen/`, a docs
site — writes an `argot.toml` for it, fits the model, and verifies argot
actually catches a foreign import. Deciding what to exclude is a judgment call an
LLM makes well; the raw `argot init` leaves it to you. `/argot-check` then scores
each diff and reads the result advisorily (never blocks); `/argot-review-pr`
reviews a whole PR against your repo's voice; `/argot-setup-ci` wires the
GitHub Action.

Prefer to drive it by hand? `argot init && argot check` runs the pipeline
directly — you make the what-to-exclude calls yourself (see
[Setup](https://argot.tmonier.com/docs/setup/)).

**More in the docs:** [Setup](https://argot.tmonier.com/docs/setup/) covers
hand-picking what argot learns from and a copy-paste prompt for any agent;
[Agents](https://argot.tmonier.com/docs/agents/) covers the skills, `AGENTS.md`,
and the optional MCP server; [The commands](https://argot.tmonier.com/docs/the-commands/)
has every flag (JSON/SARIF, ranges, `argot update`).

## Configuration

argot learns from the code *you* wrote, so it already skips tests, docs, examples,
build output, and anything it detects as generated or data-only. Exclude the rest
— vendored SDKs, generated stubs, legacy modules — in `argot.toml`'s
`[exclude].paths` (gitignore-style patterns); `argot init --suggest` finds the
generated- and data-heavy dirs for you, and you accept an intentional hit with
`argot mute <hash> --reason "…"`.
Full guide: [Configure](https://argot.tmonier.com/docs/configure/).

## What it catches

Not a replacement for ESLint, ruff, or type checkers — argot catches what they
**structurally can't**: code that's valid, typed, and lint-clean but doesn't fit
*this* repo. Three axes.

**1 · Foreign** — a pattern the repo has never used. The statistical voice model,
**98%** catch when the symbol is visible ([benchmarks](#benchmarks)):

```python
import requests                       # repo standardises on httpx       →  ! foreign · requests
_audit.insert_one({"user": uid})      # a Mongo call in a SQLAlchemy repo →  ? suspicious · call_receiver
class ReceiptView(View):              # a Django view in an all-FastAPI repo →  ! foreign · paradigm
```

**2 · Redundant** — a new function that reinvents one you already have. The
embedding index finds the original and shows you exactly where it lives:

```
  .  already implemented here (redundant)
     ↳ duplicates slugify (src/utils/text.py:1) — similarity 0.86
```

**3 · Misplaced** — the right code, filed in the wrong package:

```
  .  unusual location (misplaced)
     ↳ looks like core/downloader code filed under commands/
```

*Redundant* and *misplaced* are **advisory** — real repos hold real duplication
and cross-cutting helpers, so argot shows the nearest existing code and lets you
judge. And there's a **line it won't cross**: when a break reuses only vocabulary
you already have (a bare `ValueError` where you'd raise `HTTPException`), the
mistake is a *choice*, not a foreign pattern — argot won't gate on it, and says so.
Full, verified breakdown: [what it catches](https://argot.tmonier.com/docs/what-it-catches/).

#### Reinvention across every language — and its honest limits

The *"you already have this"* sense isn't Python-and-TypeScript-only. The scoring
is language-agnostic — identifier subtokens and callees are extracted the same way
everywhere — so every language argot parses gets it, benchmarked on **31 real repos
across 11 languages**.

**Catch is high.** Planting faithful reimplementations of a repo's own functions
(renamed, restructured) as new code, argot flags them redundant at **≥ 80% on every
corpus** (median 95%).

**False-fire is filtered, not hidden.** The naïve sense fired on 5–14% of real-commit
hunks on library/framework repos — but those fires were dominated by shapes that
*aren't* reinventions: thin wrappers, interface/family methods (a linter's `on_send`
across 271 cops), dense sibling clusters. Four cheap structural filters (body size,
symbol frequency, embedding-neighbour density — each exempted when the candidate
reuses the match's exact helpers) drop them with **zero recall loss**, taking the
clean-commit false-fire to **≤5%/hunk on 28 of 31 corpora** (3-judge labelled where it
matters). The three that remain — curl 6.2%, jellyfin 7.0%, laravel 6.6% — are
parallel backends and sibling-module methods (openssl↔wolfssl, Illuminate/\*) that a
skeptical human reviewer calls "not a reinvention, but structurally identical": the
irreducible floor of a name/structure sense with no LLM. That residual is why
*redundant* (and the quieter *misplaced*) stay **advisory** — a prompt to review,
never folded into the gated catch/over-fire numbers above. They do fire at the mildest
(`unusual`) tier, so a reinvention- or misplacement-only hunk still exits non-zero;
mute them or raise `--min-severity` to drop them from the gate. The full per-repo
catch **and false-fire** rates — for both senses — are on the
[benchmarks page](https://argot.tmonier.com/benchmarks).

### argot vs. the tools you already run

|  | Type checker | Linter | Copilot · SAST | argot |
|---|:---:|:---:|:---:|:---:|
| Catches invalid code | ✅ | ✅ | ~ | — |
| Flags what's foreign to *this* repo | ❌ | ❌ | ❌ | ✅ |
| Flags a function you **already have** | ❌ | ❌ | ❌ | ✅ |
| Flags code filed in the **wrong place** | ❌ | ❌ | ❌ | ✅ |
| Learns from *your* history · runs 100% local | ❌ | ❌ | ❌ | ✅ |

argot is additive: it sits *after* your type checker and linter and catches the
one thing they can't — code that's valid and lint-clean but unlike anything your
team has written.

## Benchmarks

**Honest, leak-free numbers.** argot has one job — flag a pattern **foreign to
the repo** (a dependency, API, or construct the codebase has never used, the kind
of thing an AI agent drags in), so the scorecard is two numbers, measured without
leakage:

- **Visible-foreign catch** — foreign imports and APIs spliced into real host
  files and judged by the real `fit` → `check` pipeline. When the foreign symbol
  is visible in the code, argot catches **565/574 (98%)**.
- **False alarm (over-fire)** — a temporal holdout (fit at an old commit, replay
  only commits the model never saw) counting how often argot fires on the repo's
  *own existing code*. Aggregate **0.22%**, worst corpus **1.17%**. A fire on a
  genuinely *new* dependency in a real commit is a **detection**, not an alarm —
  reported separately, never counted against the tool.

| Language | Corpora | Visible-foreign catch | Worst over-fire |
|---|---|---|---|
| Python | fastapi · rich · faker · saleor · wagtail · dagster · scrapy | 137/140 (98%) | 0.92% |
| TypeScript / JS | hono · ink · faker-js · excalidraw · outline · commander · express · eslint | 126/127 (99%) | 0.11% |
| Go | gh-cli · hugo | 37/38 (97%) | 1.17% |
| Rust | ripgrep · bat | 38/38 (100%) | 0.30% |
| Java | guava · junit5 | 38/38 (100%) | 0.82% |
| C# | powershell · jellyfin | 38/40 (95%) | 0.06% |
| C | redis · curl | 37/38 (97%) | 0.18% |
| C++ | rocksdb · fmt | 38/39 (97%) | 0.22% |
| Ruby | homebrew · rubocop | 38/38 (100%) | 0.63% |
| PHP | laravel · composer | 38/38 (100%) | 0.00% |

Across **31 repos in 11 languages**. Earlier published numbers were measured
train-on-test and were materially optimistic — see
[issue #92](https://github.com/get-tmonier/argot/issues/92) and the
[re-measurement evidence](docs/research/evidence/issue92-honest-rebench.md).

**What the two numbers mean.** A commit that introduces a *genuinely new*
dependency or API (a symbol with zero usage in the repo at fit time) is not an
idiomatic commit — flagging it is argot's job, so those fires are counted as
**detections**, not false alarms. The true false-alarm rate is **over-fire**:
argot firing on the repo's *own existing code*, and every one of the 31 corpora
holds it to ≤ 1.17%. The one class argot *cannot* catch is **masked foreign** —
a foreign symbol whose name collides with one the repo already uses, or a dynamic
`import()` — a documented statistical limit (~17%), since a voice model can't
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
or run `/argot-setup-ci` (see [Set up](#set-up)).

## How it works

**Two senses, both learned entirely from your git history.**

*The voice model — statistical.* A scorer runs on each diff hunk: an **import
check** (any module foreign to this repo?), a **BPE surprise** score (how much
likelier is this hunk's tokens under a generic open-source baseline than under
*your* repo?), and a **call-receiver penalty** (does it call things this kind of
file never calls?). Two frequency tables plus a callee-cluster partition — no
neural net, fits in seconds, scores in milliseconds.

*The semantic index — embeddings.* At fit, argot embeds every function with a
local **code-embedding model** (`jina-code`, ~100 MB, statically linked via
llama.cpp — CPU-first, Metal on Macs). At check, it embeds each new function and
asks two things a linter can't: *is there already one just like it?* (reinvention)
and *do its nearest neighbours live somewhere else?* (placement). No prompt, no
generation, nothing leaves your machine.

Full detail: [How it works](https://argot.tmonier.com/docs/how-it-works/) and
[The scoring model](https://argot.tmonier.com/docs/the-scoring-model/); the
experiment log is in [docs/research/](docs/research/README.md).

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
- **TypeScript** — [Hono](https://github.com/honojs/hono) · [Ink](https://github.com/vadimdemedes/ink) · [faker-js](https://github.com/faker-js/faker) · [Excalidraw](https://github.com/excalidraw/excalidraw) · [Outline](https://github.com/outline/outline)
- **JavaScript** — [Express](https://github.com/expressjs/express) · [Commander.js](https://github.com/tj/commander.js) · [ESLint](https://github.com/eslint/eslint)
- **Go** — [GitHub CLI](https://github.com/cli/cli) · [Hugo](https://github.com/gohugoio/hugo)
- **Rust** — [ripgrep](https://github.com/BurntSushi/ripgrep) · [bat](https://github.com/sharkdp/bat)
- **Java** — [Guava](https://github.com/google/guava) · [JUnit 5](https://github.com/junit-team/junit5)
- **C#** — [PowerShell](https://github.com/PowerShell/PowerShell) · [Jellyfin](https://github.com/jellyfin/jellyfin)
- **C** — [redis](https://github.com/redis/redis) · [curl](https://github.com/curl/curl)
- **C++** — [RocksDB](https://github.com/facebook/rocksdb) · [fmt](https://github.com/fmtlib/fmt)
- **Ruby** — [Homebrew](https://github.com/Homebrew/brew) · [RuboCop](https://github.com/rubocop/rubocop)
- **PHP** — [Laravel](https://github.com/laravel/framework) · [Composer](https://github.com/composer/composer)

Built on [tree-sitter](https://tree-sitter.github.io/tree-sitter/) and its
per-language grammars (Python, TypeScript, JavaScript, Go, Rust, C, C++, Java,
C#, PHP, Ruby), [libgit2](https://libgit2.org/) via [git2](https://docs.rs/git2/) (vendored,
no network transports), HuggingFace
[tokenizers](https://github.com/huggingface/tokenizers) (UnixCoder BPE),
[clap](https://docs.rs/clap/), [Serde](https://serde.rs/), and
[cargo-dist](https://opensource.axo.dev/cargo-dist/) /
[axoupdater](https://github.com/axodotdev/axoupdater) for releases and
`argot update`. The semantic layer links [llama.cpp](https://github.com/ggml-org/llama.cpp)
(MIT) statically via [`llama-cpp-2`](https://crates.io/crates/llama-cpp-2).

The semantic layer's code-embedding model is
[**jina-embeddings-v2-base-code**](https://huggingface.co/jinaai/jina-embeddings-v2-base-code)
by [Jina AI](https://jina.ai/), used under the **Apache License 2.0**. argot
fetches it on first use and redistributes the `Q4_K_M` GGUF quantization (a
derivative work under Apache-2.0 §4 — weights quantized, architecture unchanged)
from its [`semantic-model-v1`](https://github.com/get-tmonier/argot/releases/tag/semantic-model-v1)
release. argot is not affiliated with, nor endorsed by, Jina AI.

## License

MIT
