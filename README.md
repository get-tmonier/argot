<p align="center">
  <img src="docs/argot-logo.svg" alt="argot" width="200" />
</p>

<p align="center">
  <strong>Your codebase has a voice. argot makes AI code speak it.</strong><br/>
  <em>A local guardrail that catches AI-written code that doesn't fit your repo — a dependency you've never used, a function you already wrote, logic in the wrong place, an import that breaks your layering, or a test quietly weakened to make a failing suite green. Learned from your git history. No LLM, no cloud, no GPU.</em>
</p>

<p align="center">
  <a href="https://argot.tmonier.com"><strong>argot.tmonier.com</strong></a>
  &nbsp;·&nbsp;
  <a href="https://argot.tmonier.com/docs/">Documentation</a>
  &nbsp;·&nbsp;
  <a href="https://argot.tmonier.com/benchmarks">Benchmarks</a>
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
  &nbsp;·&nbsp;<a href="https://argot.tmonier.com/docs/languages/">11 languages →</a>
</p>

---

Type checkers ask *"is this valid?"* argot asks the question that used to live in code review: *"is this how **we** do it here?"* — and catches AI code that's flawless, type-correct, lint-clean, and still doesn't belong.

It also asks a second question no other tool asks: **did the AI play fair?** When an agent can't make a failing test pass, the cheapest path to "done" is to make the test stop looking. argot reads both sides of every diff and pairs a weakened, disabled, or deleted test with the production change it covers.

### Five learned detectors — plus the rules only your repo could write

| | Rule | It catches | |
| :-- | :-- | :-- | :-- |
| 🚫 | **`foreign-import`** + friends | a dependency, API, or idiom your repo has **never used** | *"we don't do it this way here"* |
| ♻️ | **`redundant`** | a new function that **reinvents one you already have** | *"you already have this"* |
| 📍 | **`misplaced`** | the right code, filed in the **wrong place** | *"this doesn't belong here"* |
| 🧱 | **`layering`** | an internal import that **reverses your architecture** | *"we never cross this boundary"* |
| 🧪 | **`test-deleted`** + friends | a test **quietly weakened, disabled, or removed** alongside the prod change it covers | *"don't game the tests"* |
| 📜 | **your own rules** | the conventions **only your repo has** — scripted, no recompile | *"here's exactly how we do it"* |

The first five are learned from your git history. The sixth is [written by you](#your-conventions-as-rules) — and it's the part of every linter config your team actually cares about.

## Get started

```sh
# install (single static binary — no Python, no Node)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh
```

Windows: `powershell -c "irm https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.ps1 | iex"` · npm: `npm install -g @tmonier/argot`

**Sixty seconds of proof, zero setup, on your own history:**

```sh
argot audit      # ⏪ what did AI sneak into your last 50 commits?
```

`audit` fits the voice as it was 50 commits ago (in a temp worktree — your tree stays untouched), rescores everything since, and attributes every finding to its introducing commit — **ai-assisted / human / unknown**, from concrete commit markers only:

```
━━ argot audit ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  last 50 commits · 52% carry AI markers · 1 finding would have met review

  Worst offender — commit cae8349 · ai-assisted
  ! landing/src/pages/llms-full.txt.ts:L1-32 · foreign-import
      ↳ astro (L1), astro:content (L2) — 0 of 49 module specifiers…
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

Then fit today's voice so `check` raises these *before* they merge:

```sh
argot init       # learn this repo's voice (~25 s on a 1,100-file repo)
argot check      # score your working changes against it
```

Accuracy is a function of setup — argot learns from what it's allowed to see. Best path: `npx skills add get-tmonier/argot`, then `/argot-setup` in your coding agent (Claude Code, Cursor, 70+ agents) reads your repo, excludes what shouldn't shape the voice, and verifies the catch. Full guide: [Setup](https://argot.tmonier.com/docs/setup/) · [Getting started](https://argot.tmonier.com/docs/getting-started/).

## Demo

<p align="center">
  <img src="docs/demo/demo.gif" alt="argot check flagging a foreign Django-style view in an all-FastAPI codebase" width="760" />
</p>

A PR adds a **Django-style view** to an all-FastAPI codebase. mypy and ruff are silent — the framework it reaches for is one this repo has never imported:

```
argot check · 1 hunk above threshold (1 foreign)

fastapi/receipts.py
  !  L1-L10         1.00  foreign  · staged · foreign-import [94a92c256ea1]
     ↳ django (L1) — 0 of 74 module specifiers in repo
       common here: fastapi (357×), pydantic (129×), typing (129×) (+7 more)
  1 | from django.views import View
             ^^^^^^
```

`redundant` names the function you already have (`↳ duplicates slugify (src/utils/text.py:14) — similarity 0.86`), `misplaced` names where the code belongs, `layering` names the direction an import reverses, and every `↳` line is your repo's own evidence. Full anatomy: [Reading the output](https://argot.tmonier.com/docs/reading-the-output/) · [What it catches](https://argot.tmonier.com/docs/what-it-catches/).

## Your conventions, as rules

Every team has conventions no generic linter ships: *"config goes through the loader, never raw `process.env`"*, *"retries use the backoff helper, not a sleep in a loop"*, *"we log, never print"*. They live in review comments and onboarding docs — until an AI agent, who read neither, merges around them. With argot they're **repo-local rules**: a TOML manifest + a small sandboxed script in `.argot/rules/`, versioned with your code, loaded at run time — no plugin build, no recompile, one rule format across all 11 languages.

```toml
# .argot/rules/no-direct-env/rule.toml
[rule]
schema = 1
name = "no-direct-env"
description = "config is read through loadConfig() — raw process.env skips validation and defaults"
severity = "warn"
languages = ["typescript", "javascript"]
```

```rhai
// check.rhai — flag process.env reads outside the config module
if file.path.contains("src/config") { return; }
for m in ts_query("(member_expression) @e") {
    if m.text.starts_with("process.env") {
        report(m.line, "read config through loadConfig() — raw process.env skips validation and defaults (see src/config.ts)");
    }
}
```

Rules run on **changed files only** — adopting one creates zero backlog noise, it just guards new code — and their findings are suppressed, configured, and rendered exactly like built-in rules. Because argot fits your history, a rule can even ask it questions: `import_attested("moment")` means *"has this repo ever used this date library?"* — something no history-blind tool can express. `argot rules test` is the red/green authoring loop. Full reference + worked example: [Custom rules](https://argot.tmonier.com/docs/custom-rules/).

## Configure it like any linter

Every rule (built-in or yours) defaults through `argot.toml [rules]` — `error` / `warn` / `off`, per rule or per group — or per run via `--rule layering=warn`. Excludes are gitignore-style `[exclude].paths`; inline `# argot: ignore-next-line rule=… — reason` and `argot mute <hash>` give line-level and durable committed acceptances. Guides: [Configure](https://argot.tmonier.com/docs/configure/) · [The commands](https://argot.tmonier.com/docs/the-commands/).

### argot vs. the tools you already run

|  | Type checker | Linter | Copilot · SAST | argot |
|---|:---:|:---:|:---:|:---:|
| Catches invalid code | ✅ | ✅ | ~ | — |
| Flags what's foreign to *this* repo | ❌ | ❌ | ❌ | ✅ |
| Flags a function you **already have** | ❌ | ❌ | ❌ | ✅ |
| Flags code filed in the **wrong place** | ❌ | ❌ | ❌ | ✅ |
| Flags an import that **breaks your layering** | ❌ | ❌ | ❌ | ✅ |
| Flags a test **quietly weakened to game a failing suite** | ❌ | ❌ | ❌ | ✅ |
| Enforces **your team's own conventions**, cross-language, on the diff | ❌ | ~ | ❌ | ✅ |
| Audits merged history · **attributes findings AI vs human** | ❌ | ❌ | ❌ | ✅ |
| Learns from *your* history · runs 100% local | ❌ | ❌ | ❌ | ✅ |

argot is additive: it sits *after* your type checker and linter and catches the one thing they can't — code that's valid and lint-clean but unlike anything your team has written.

## Benchmarks

**Honest, leak-free numbers**, measured by the real `fit → check` pipeline — foreign fixtures spliced into real host files; false alarms counted on a temporal holdout the model never saw:

- **Foreign catch — 604/618 (98%)** when the foreign symbol is visible in the diff · **false alarms 0.22%** of 22,785 real hunks (worst corpus 1.17%)
- **Architecture — 244/252 (96.8%)** caught · **0/140** controls flagged
- **Reinvention — median 94%** · **Misplacement — 86–99%** where the repo has separable architecture
- **Test-integrity — 144/153 (94.1%)** gaming tactics caught · **0/102** legitimate-refactor controls · 1.24% of 3,540 replayed accepted commits flagged

One documented limit: **masked foreign** — a foreign symbol whose name collides with one you already use — is statistically invisible to a voice model. We publish that number rather than hide it. Per-language and per-corpus tables, methodology, confidence intervals: [benchmarks page](https://argot.tmonier.com/benchmarks) (CI-fed, can't drift from what ships). Want a language validated? [Open an issue](https://github.com/get-tmonier/argot/issues/new).

## CI

```yaml
- uses: get-tmonier/argot@main   # non-blocking voice score on every PR
```

`--format github` prints inline PR annotations; `--format sarif` feeds code scanning; `--format json` is a stable schema. Copy-paste setups incl. pre-commit: [the CI guide](https://argot.tmonier.com/docs/ci/).

## How it works

Five detectors, one source of truth — your git history. A statistical voice model (two frequency tables + a callee-cluster partition — no neural net) catches foreign imports, callees, and token shapes; a local code-embedding model (jina-code via statically-linked llama.cpp) catches reinvention and misplacement; a module-dependency graph catches layering reversals; a test-inventory diff catches gamed tests. Fit in seconds, check in milliseconds, nothing leaves your machine. Full detail: [How it works](https://argot.tmonier.com/docs/how-it-works/) · [The scoring model](https://argot.tmonier.com/docs/the-scoring-model/) · [Performance](https://argot.tmonier.com/docs/performance/) · experiment log in [docs/research/](docs/research/README.md).

## Contributing

Issues and PRs welcome — start with [CONTRIBUTING.md](CONTRIBUTING.md):

```sh
git clone https://github.com/get-tmonier/argot && cd argot
just build       # cargo build --release -p argot → target/release/argot
just verify      # cargo fmt --check + clippy -D warnings + cargo test
```

## Acknowledgements

argot is benchmarked against real repositories used as **read-only corpora** — cloned at benchmark time, never redistributed, each under its own license, none affiliated with argot: FastAPI, rich, faker, Saleor, Wagtail, Dagster, Scrapy, Hono, Ink, faker-js, Excalidraw, Outline, Express, Commander.js, ESLint, GitHub CLI, Hugo, ripgrep, bat, Guava, JUnit 5, PowerShell, Jellyfin, redis, curl, RocksDB, fmt, Homebrew, RuboCop, Laravel, and Composer.

Built on [tree-sitter](https://tree-sitter.github.io/tree-sitter/) (11 grammars), [libgit2](https://libgit2.org/) via [git2](https://docs.rs/git2/), HuggingFace [tokenizers](https://github.com/huggingface/tokenizers) (UnixCoder BPE), [Rhai](https://rhai.rs/) (scripted rules), [clap](https://docs.rs/clap/), [Serde](https://serde.rs/), and [cargo-dist](https://opensource.axo.dev/cargo-dist/). The semantic layer links [llama.cpp](https://github.com/ggml-org/llama.cpp) (MIT) statically via [`llama-cpp-2`](https://crates.io/crates/llama-cpp-2); its model is [**jina-embeddings-v2-base-code**](https://huggingface.co/jinaai/jina-embeddings-v2-base-code) by [Jina AI](https://jina.ai/) (Apache-2.0), fetched on first use as a `Q4_K_M` GGUF quantization (a derivative work under Apache-2.0 §4) from the [`semantic-model-v1`](https://github.com/get-tmonier/argot/releases/tag/semantic-model-v1) release. argot is not affiliated with, nor endorsed by, Jina AI.

## License

MIT
