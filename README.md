<p align="center">
  <img src="docs/argot-logo.svg" alt="argot" width="200" />
</p>

<p align="center">
  <strong>Like ESLint, but for the unwritten rules.</strong><br/>
  <em>argot learns your repo's voice from its own git history, then flags code that doesn't sound like your team wrote it.</em>
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
  <a href="https://github.com/get-tmonier/argot/blob/main/LICENSE"><img src="https://img.shields.io/github/license/get-tmonier/argot" alt="License" /></a>
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
  &nbsp;·&nbsp;<a href="#supported-languages">10 languages →</a>
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
Intel) and Linux (x64 + arm64). See [docs/ci.md](docs/ci.md) and the
[install docs](https://argot.tmonier.com/docs/) for the full platform matrix.

## Quickstart

```sh
cd your-repo
argot extract      # walk git history → .argot/dataset.jsonl
argot fit          # build the corpus + baseline, then calibrate the threshold
argot check        # score uncommitted changes (or pass a ref/range)
```

Run `fit` once per repo (and after major refactors); run `check` on every diff —
`--staged`, a `HEAD~5..HEAD` range, `--commit <sha>`, `--min-severity foreign`,
or `--format json|sarif` for machines. `argot inspect` reports corpus health and
a Ready / Marginal / Not-recommended verdict; `argot update` pulls the latest
release. Full reference: `argot --help` and the
[docs site](https://argot.tmonier.com/docs/).

## What it catches

It does *not* replace ESLint, ruff, or type checkers — it catches what they
can't: code that's **technically fine but socially wrong** for this project. Each
example below is valid, fully typed, lint-clean, and passes `mypy strict`; every
other tool in your CI is silent on it. (These are real fixtures from the
FastAPI benchmark catalog, where argot catches 32/32 under the honest
protocol — how reliably each *class* fires varies a lot by language and
corpus; the per-language table below reports the real rates.)

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

## Supported languages — honest numbers

All numbers below are **leak-free**: false positives come from a temporal
holdout (fit at an old commit, replay only commits the model never saw, split
by whether the file existed at fit time), and recall comes from curated break
fixtures spliced into real files and judged by the real `fit` → `check`
pipeline. Earlier published numbers were measured train-on-test and were
materially optimistic — see
[issue #92](https://github.com/get-tmonier/argot/issues/92) and the
[re-measurement evidence](docs/research/evidence/issue92-honest-rebench.md).
FP is per-corpus with bootstrap 95% CIs in the evidence doc; recall is on
deliberately *hard* fixture classes (wrong error discipline, wrong
concurrency model, API misuse within libraries the repo already uses, naming
shape — not just foreign imports).

| Language | Extensions | FP on edits to existing files | FP on new files | Recall (hard curated breaks) |
|---|---|---|---|---|
| Python | `.py` | 0.0–6.6% (4 of 5 corpora ≤ 2.8%; fastapi 6.6%) | 3.1–3.9% | rich 69% · fastapi/faker/saleor/wagtail 100% |
| TypeScript | `.ts` `.tsx` | 0.1–3.5% | 0–21% (excalidraw worst) | 47–88% |
| JavaScript | `.js` `.jsx` | uses the TypeScript adapter | | |
| Go | `.go` | 2.3% / 5.9% | 7.8–13% | gh-cli **38%** |
| Rust | `.rs` | ripgrep 1.0% · bat **11.5%** | 7.7% (thin sample) | ripgrep **31%** |
| Java | `.java` | guava 2.1% · junit5 0.0% | 0.0% | guava **57%** |
| C# | `.cs` | powershell 1.1% · jellyfin **9.7%** | 14% (jellyfin) | powershell **54%** |
| C | `.c` `.h` | redis 0.7% · curl 0.3% | redis **61%** (19/31 hunks) | redis **21%** |
| C++ | `.cpp` `.cc` `.hpp` | rocksdb **6.2%** · fmt 3.2% | rocksdb **49%** | rocksdb **23%** |
| Ruby | `.rb` | homebrew 4.6% · rubocop 3.5% | 4.7% | homebrew **39%** |
| PHP | `.php` | laravel 0.8% · composer 0.0% | 3.8–11.5% | laravel **62%** |

**What this means in practice.** argot's reliable value today is the
tripwire class: foreign imports and strongly foreign API surfaces fire at
low false-positive cost on most corpora. The *hard* classes it aspires to —
in-vocabulary breaks like a bare `throw new Exception` in a typed-error
codebase — are caught well on the mature Python corpora but **miss more
often than they hit in most other languages**, and a handful of corpora
(bat, jellyfin, rocksdb, hugo, fastapi edits; new files on
excalidraw/redis/rocksdb) still exceed our ≤2%-existing / ≤5%-new-file
false-positive gates, driven by the call-receiver cluster stage. We publish
those numbers red rather than tune the benchmark until they look green;
closing the gap needs a structurally smarter scorer (tracked in the
[research log](docs/research/README.md)).

Mixed-language monorepos calibrate one threshold per language and dispatch
each hunk by file extension. Live per-corpus results at
[argot.tmonier.com/benchmarks](https://argot.tmonier.com/benchmarks) (fed
from CI via `argot-bench --mode honest`), with methodology in
[benchmarks/README.md](benchmarks/README.md).

**Adding a language is a roadmap item, not an architectural blocker.** The
scoring pipeline is language-agnostic; per-language is just a tree-sitter
adapter. We ship a language only *after* benchmarking it honestly on real
corpora — and we publish the numbers it actually gets. Want a corpus
validated? [Open an issue](https://github.com/get-tmonier/argot/issues/new).

## Running in CI

`argot check` emits `--format json` (stable schema) and `--format sarif`
(SARIF 2.1.0 for GitHub code scanning). A composite GitHub Action ships at the
repo root (`uses: get-tmonier/argot@main`), and `.pre-commit-hooks.yaml`
registers an `argot-check` hook. Copy-paste setups: [docs/ci.md](docs/ci.md).

For LLM coding agents, `argot mcp` runs a Model Context Protocol server so an
agent can ask for the repo's voice *before* generating and score hunks *after* —
setup for Claude Code, Cursor, and generic clients in [docs/mcp.md](docs/mcp.md).

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
