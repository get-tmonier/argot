<p align="center">
  <img src="docs/argot-logo.svg" alt="argot" width="200" />
</p>

<p align="center">
  <strong>Your codebase has a voice. argot makes AI code speak it.</strong><br/>
  <em>A local guardrail that catches AI-written code that doesn't fit your repo — a dependency you've never used, a function you already wrote, logic in the wrong place, an import that breaks your layering, or a test quietly weakened to make a failing suite green. Learned from your git history. No LLM, no cloud, no GPU.</em><br/><br/>
  <strong><code>argot audit</code> — one command, zero setup — scores your recent history and tells you who wrote each offender: ai-assisted, human, or unknown.</strong>
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
  <img src="https://img.shields.io/badge/PHP-777BB4?logo=php&logoColor=white" alt="PHP" />
  &nbsp;·&nbsp;<a href="#benchmarks">11 languages →</a>
</p>

---

Type checkers ask *"is this valid?"* argot asks the question that used to live in code review: *"is this how **we** do it here?"* — and catches AI code that's flawless, type-correct, lint-clean, and still doesn't belong.

And it asks a second question no other tool asks: **did the AI play fair?** When an agent can't make a failing test pass, the cheapest path to "done" is to make the test stop looking — skip it with a plausible reason, delete the assertion that fails, nudge the expected value. The diff looks tidy, CI turns green, and your safety net now has a hole exactly where the code is newest. argot reads both sides of every diff and pairs a weakened, disabled, or deleted test with the production change it covers — before you merge on a checkmark that no longer means anything.

### Five detectors, all learned from your git history

| | Rule | It catches | |
| :-- | :-- | :-- | :-- |
| 🚫 | **`foreign-import`** + friends | a dependency, API, or idiom your repo has **never used** | *"we don't do it this way here"* |
| ♻️ | **`redundant`** | a new function that **reinvents one you already have** | *"you already have this"* |
| 📍 | **`misplaced`** | the right code, filed in the **wrong place** | *"this doesn't belong here"* |
| 🧱 | **`layering`** | an internal import that **reverses your architecture** | *"we never cross this boundary"* |
| 🧪 | **`test-deleted`** + friends | a test **quietly weakened, disabled, or removed** alongside the prod change it covers | *"don't game the tests"* |

Copilot, ESLint, SAST — every tool judges by one *global* idea of good code. argot learns **yours** and judges each AI diff against it. Configure it like any linter: every rule defaults to `error` (`test-weakened` ships `warn` — see below); downgrade or disable any of them.

- 📊 **98%** foreign catch (604/618) · **0.22%** false alarms (49 of 22,785 real hunks) — [honest, leak-free benchmarks](#benchmarks) on 31 repos, 11 languages
- 🧱 **96.8%** architecture-violation recall (244/252) at **0%** false positives (0/140 control edits)
- 🧪 **94.1%** test-gaming catch (144/153 authored fixtures) at **1.24%** gating false-alarm rate on replayed accepted history (0/102 legit-refactor controls fired)
- ⏪ **`argot audit`** — the history scorecard: % of commits carrying AI markers + every finding attributed to its introducing commit (ai-assisted / human / unknown, [concrete markers only](https://argot.tmonier.com/docs/the-commands/#audit) — never style); terminal, JSON, markdown, or shareable HTML
- ⚡ **Rust · single static binary** — checks a diff in ~0.2 s (0.6 s when it defines new functions); the one-time fit is ~25 s on a 1,100-file repo, ~4 s to refresh (measured on FastAPI, laptop CPU). Scales to monorepos: a machine-wide embedding cache + multi-core calibration bring a ~30k-function repo's warm `fit` to ~2.7 min and a seeded `audit` to ~2.3 min — [byte-identical findings, cold or warm](https://argot.tmonier.com/docs/performance/)
- 🧠 **A local code-embedding model** (`jina-code`, ~100 MB, CPU-first, Metal on Macs) — semantic understanding from an encoder, **not an LLM**: no generation, no API key, no GPU
- 🔒 **Nothing leaves your machine** — no telemetry, no account; one cached version check per day (opt-out) is the only network call it ever makes on its own

## Get started

```sh
# install (single static binary — no Python, no Node)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh
```

Windows: `powershell -c "irm https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.ps1 | iex"` · npm: `npm install -g @tmonier/argot`

**Then — sixty seconds of proof, on your own history:**

```sh
argot audit      # ⏪ what did AI sneak into your last 50 commits?
```

`audit` runs on a fresh clone with zero setup: it fits the voice **as it was
50 commits ago** (in a temp worktree — your tree stays untouched), rescores
everything since, and attributes every finding to its introducing commit —
**ai-assisted / human / unknown**, from concrete commit markers only (agent
`Co-authored-by` trailers, bot authors — never style). Real run on argot's
own history:

```
━━ argot audit ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  last 50 commits · 2026-06-01 → 2026-07-11 · 50 commits audited
  52% carry AI markers (26 of 50) · 1 finding would have met review

  voice    1  code foreign to how this repo writes

  Worst offender — commit cae8349 · ai-assisted
  ! landing/src/pages/llms-full.txt.ts:L1-32 · foreign-import
      "feat: v1 launch polish — on…" — Co-Authored-By: Claude Opus 4.8
      ↳ astro (L1), astro:content (L2), #lib/site (L3) — 0 of 49 module s…

  Merged code is accepted code — read each finding as "would have
  prompted review before merge", not a bug list. "human" means no AI
  markers were found; the AI share is a floor, not a census.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

`--since 6m` audits by time, `--format json|markdown|html` gives a stable
schema, a PR-pasteable card, or a self-contained shareable page. Then
`argot init` fits today's voice so `argot check` raises these before they
merge:

```sh
argot init       # learn this repo's voice (~25 s on a 1,100-file repo)
argot check      # score your working changes against it
```

(Everything argot writes in-repo is gitignored except the small `argot.toml` —
the model and index live in `.argot/` and `~/.cache/argot`, never in your git.)

**Before you rely on it: the setup calls.** argot's accuracy is a function of
its setup — it learns from what it's allowed to see, and a fit that ingests
vendored SDKs, generated stubs, or data files speaks with the wrong voice.
Recommended: let your coding agent make those calls —

```sh
npx skills add get-tmonier/argot     # then run /argot-setup in your agent
```

`/argot-setup` (Claude Code, Cursor, 70+ agents) reads your codebase, decides
what shouldn't shape the voice, writes `argot.toml`, re-fits, and verifies
argot actually catches a foreign import. Then `/argot-check` scores each diff,
`/argot-review-pr` reviews a whole PR, `/argot-setup-ci` wires the GitHub
Action. By hand instead: `argot init --suggest`, review, add to `argot.toml
[exclude].paths`, re-run `argot init` — and argot itself tells you when to
revisit (every fit re-scans for new generated/data-heavy directories; `argot
status` is the health view). After that the voice maintains itself: when your
default branch gains enough new source, `check` re-fits in the background —
from **accepted history only**, so a feature branch's unmerged commits (the
code being judged) never train the judge.

## Demo

<p align="center">
  <img src="docs/demo/demo.gif" alt="argot check flagging a foreign Django-style view in an all-FastAPI codebase" width="760" />
</p>

A PR adds a **Django-style view** to an all-FastAPI codebase. mypy and ruff are
silent — the framework it reaches for is one this repo has never imported:

```
argot check · 1 hunk above threshold (1 foreign)
note: argot is a probabilistic style linter — verify before action.

fastapi/receipts.py
  !  L1-L10         1.00  foreign  · staged · foreign-import [94a92c256ea1]
     ↳ django (L1) — 0 of 74 module specifiers in repo
       common here: fastapi (357×), pydantic (129×), typing (129×) (+7 more)
  1 | from django.views import View
             ^^^^^^
  2 | from django.http import JsonResponse, HttpResponseNotFound
        (+8 more lines)
```

The `redundant` rule goes further — it names the code you already have:

```
src/text/slug.py
  .  L12-L24        0.86  unusual  · staged · redundant [c2117f8ab90d]
     ↳ duplicates slugify (src/utils/text.py:14) — similarity 0.86
```

`misplaced` names where the code actually belongs:

```
src/cli/commands/fetch.py
  .  L18-L41        0.62  unusual  · staged · misplaced [3e51b7c20d6f]
     ↳ looks like core/downloader code filed under cli/commands
```

And `layering` flags the import that quietly reverses your architecture:

```
core/parser.py
  .  L3             1.00  unusual  · staged · layering [77d1e02c433a]
     ↳ cli → core is this repo's direction — this import reverses it
```

The glyph grades confidence (`!` foreign · `?` suspicious · `.` unusual), the
`[hash]` is a stable id you can `argot mute`, and every `↳` line is your repo's
own evidence. Full anatomy: [Reading the output](https://argot.tmonier.com/docs/reading-the-output/).

## Configure it like any linter

`argot rules` lists every rule. All of them default to `error`; set any rule —
or a whole group — to `warn` or `off` in `argot.toml`, or per run:

```toml
[rules]
misplaced = "warn"     # report, but don't fail the check
semantic  = "off"      # disable the whole embedding-based group
```

```sh
argot check --rule layering=warn --error-on-warnings
```

Excludes are just as boring: `[exclude].paths` (gitignore-style; `argot init
--suggest` finds candidates), inline `# argot: ignore-next-line rule=redundant —
reason`, and `argot mute <hash> --reason "…"` for durable, committed
acceptances. Full guides: [Setup](https://argot.tmonier.com/docs/setup/) ·
[Configure](https://argot.tmonier.com/docs/configure/).

### argot vs. the tools you already run

|  | Type checker | Linter | Copilot · SAST | argot |
|---|:---:|:---:|:---:|:---:|
| Catches invalid code | ✅ | ✅ | ~ | — |
| Flags what's foreign to *this* repo | ❌ | ❌ | ❌ | ✅ |
| Flags a function you **already have** | ❌ | ❌ | ❌ | ✅ |
| Flags code filed in the **wrong place** | ❌ | ❌ | ❌ | ✅ |
| Flags an import that **breaks your layering** | ❌ | ❌ | ❌ | ✅ |
| Flags a test **quietly weakened to game a failing suite** | ❌ | ❌ | ❌ | ✅ |
| Audits merged history · **attributes findings AI vs human** | ❌ | ❌ | ❌ | ✅ |
| Learns from *your* history · runs 100% local | ❌ | ❌ | ❌ | ✅ |

argot is additive: it sits *after* your type checker and linter and catches the
one thing they can't — code that's valid and lint-clean but unlike anything your
team has written.

## Benchmarks

**Honest, leak-free numbers**, measured by the real `fit → check` pipeline —
foreign fixtures spliced into real host files; false alarms counted on a
temporal holdout (fit at an old commit, replay only commits the model never saw).

- **Foreign catch — 604/618 (98%)** when the foreign symbol is visible in the diff.
- **False alarms — 0.22%** of 22,785 real hunks of the repos' own code; worst corpus **1.17%**. A fire on a genuinely *new* dependency in a real commit is a **detection**, reported separately — never counted against the tool.
- **Architecture — 244/252 (96.8%)** planted layering violations caught, **0/140** control edits flagged, worst over-fire 2.7% (23 corpora).
- **Reinvention — 85–100%** per corpus (median 94%) · false-fire ≤ 2.8% of hunks. **Misplacement — 86–99%** where the repo has separable architecture · ≤ 1.5%.
- **Test-integrity — 144/153 (94.1%)** authored gaming tactics caught across 22 corpora / 11 languages, **0/102** legitimate-refactor controls fired, **1.24%** of 3,540 replayed accepted test-touching commits flagged at gating severity — [evidence](docs/research/evidence/test-integrity-capstone.md).

| Language | Corpora | Visible-foreign catch | Worst over-fire |
|---|---|---|---|
| Python | fastapi · rich · faker · saleor · wagtail · dagster · scrapy | 145/151 (96%) | 0.92% |
| TypeScript | hono · ink · faker-js · excalidraw · outline | 102/105 (97%) | 0.11% |
| JavaScript | express · commander · eslint | 48/48 (100%) | 0.00% |
| Go | gh-cli · hugo | 37/38 (97%) | 1.17% |
| Rust | ripgrep · bat | 38/38 (100%) | 0.30% |
| Java | guava · junit5 | 38/38 (100%) | 0.82% |
| C# | powershell · jellyfin | 40/42 (95%) | 0.06% |
| C | redis · curl | 37/38 (97%) | 0.18% |
| C++ | rocksdb · fmt | 40/41 (98%) | 0.22% |
| Ruby | homebrew · rubocop | 38/38 (100%) | 0.63% |
| PHP | laravel · composer | 41/41 (100%) | 0.00% |

One documented limit: **masked foreign** — a foreign symbol whose name collides
with one you already use — is statistically invisible to a voice model (~17%
of the hardest fixtures). We publish that number rather than hide it. Full
per-corpus tables, methodology, and confidence intervals:
[benchmarks page](https://argot.tmonier.com/benchmarks) (fed from CI, can't
drift from what ships). Earlier train-on-test numbers were retracted —
[issue #92](https://github.com/get-tmonier/argot/issues/92).

Want a language or corpus validated? The pipeline is language-agnostic —
per-language is a tree-sitter adapter, shipped only after honest benchmarks.
[Open an issue](https://github.com/get-tmonier/argot/issues/new).

## CI

```yaml
- uses: get-tmonier/argot@main   # non-blocking voice score on every PR
```

`argot check --format github` prints inline PR annotations directly;
`--format sarif` feeds GitHub code scanning; `--format json` is a stable
schema for anything else. `.pre-commit-hooks.yaml` registers an `argot-check`
hook, and `argot model fetch` pre-warms the embedding model in CI images.
Copy-paste setups: [the CI guide](https://argot.tmonier.com/docs/ci/).

## How it works

**Five detectors, one source of truth: your git history.**

1. *Voice* (`foreign-import` · `unfamiliar-callee` · `rare-tokens` · `convention`) —
   a statistical scorer per diff hunk: is any module foreign to this repo? how
   much likelier are these tokens under a generic open-source baseline than
   under *yours*? does it call things this kind of file never calls? Two
   frequency tables and a callee-cluster partition — no neural net, fits in
   seconds, scores in milliseconds.
2. *Reinvention* (`redundant`) — at fit, argot embeds every function with a
   local code-embedding model (`jina-code`, statically linked llama.cpp). At
   check it asks: *is there already one just like this?* — and self-calibrates
   against your own history so repos with legitimate parallel code (per-locale
   providers, protocol variants) don't drown in noise.
3. *Placement* (`misplaced`) — *do this function's nearest neighbours all live
   somewhere else?* argot learns your real package granularity and abstains
   entirely on repos with no separable architecture.
4. *Architecture* (`layering`) — a module-dependency graph of your imports; a
   diff that reverses an established layer direction or crosses a boundary the
   repo never crosses gets flagged, with **0%** false positives on control edits.
5. *Integrity* (`test-deleted` · `test-disabled` · `test-weakened`) — at fit, a
   mini-replay of your accepted history learns which test-editing patterns are
   normal here; at check, a test quietly deleted, skipped, or weakened
   alongside the production code it exercised is flagged — the complement of
   the other four, which catch foul play with *code*; this one catches an AI
   agent gaming *tests* to green a failing suite.

No prompt, no generation, nothing leaves your machine. Full detail:
[How it works](https://argot.tmonier.com/docs/how-it-works/) ·
[The scoring model](https://argot.tmonier.com/docs/the-scoring-model/) ·
experiment log in [docs/research/](docs/research/README.md).

## Contributing

Issues and PRs welcome — start with [CONTRIBUTING.md](CONTRIBUTING.md) (`rustup`
+ `just`, the `just verify` gate, proposing a new language or corpus).

```sh
git clone https://github.com/get-tmonier/argot && cd argot
just build       # cargo build --release -p argot → target/release/argot
just verify      # cargo fmt --check + clippy -D warnings + cargo test
```

## Acknowledgements

argot's scorer is only as honest as the corpora it's benchmarked against. We use
these repositories as **read-only benchmark corpora** — cloned at benchmark time,
never redistributed. None are affiliated with argot; each remains under its own
license.

- **Python** — [FastAPI](https://github.com/tiangolo/fastapi) · [rich](https://github.com/Textualize/rich) · [faker](https://github.com/joke2k/faker) · [Saleor](https://github.com/saleor/saleor) · [Wagtail](https://github.com/wagtail/wagtail) · [Dagster](https://github.com/dagster-io/dagster) · [Scrapy](https://github.com/scrapy/scrapy)
- **TypeScript** — [Hono](https://github.com/honojs/hono) · [Ink](https://github.com/vadimdemedes/ink) · [faker-js](https://github.com/faker-js/faker) · [Excalidraw](https://github.com/excalidraw/excalidraw) · [Outline](https://github.com/outline/outline)
- **JavaScript** — [Express](https://github.com/expressjs/express) · [Commander.js](https://github.com/tj/commander.js) · [ESLint](https://github.com/eslint/eslint)
- **Go** — [GitHub CLI](https://github.com/cli/cli) · [Hugo](https://github.com/gohugoio/hugo) &nbsp;·&nbsp; **Rust** — [ripgrep](https://github.com/BurntSushi/ripgrep) · [bat](https://github.com/sharkdp/bat) &nbsp;·&nbsp; **Java** — [Guava](https://github.com/google/guava) · [JUnit 5](https://github.com/junit-team/junit5)
- **C#** — [PowerShell](https://github.com/PowerShell/PowerShell) · [Jellyfin](https://github.com/jellyfin/jellyfin) &nbsp;·&nbsp; **C** — [redis](https://github.com/redis/redis) · [curl](https://github.com/curl/curl) &nbsp;·&nbsp; **C++** — [RocksDB](https://github.com/facebook/rocksdb) · [fmt](https://github.com/fmtlib/fmt)
- **Ruby** — [Homebrew](https://github.com/Homebrew/brew) · [RuboCop](https://github.com/rubocop/rubocop) &nbsp;·&nbsp; **PHP** — [Laravel](https://github.com/laravel/framework) · [Composer](https://github.com/composer/composer)

Built on [tree-sitter](https://tree-sitter.github.io/tree-sitter/) (11 grammars),
[libgit2](https://libgit2.org/) via [git2](https://docs.rs/git2/), HuggingFace
[tokenizers](https://github.com/huggingface/tokenizers) (UnixCoder BPE),
[clap](https://docs.rs/clap/), [Serde](https://serde.rs/), and
[cargo-dist](https://opensource.axo.dev/cargo-dist/).
The semantic layer links [llama.cpp](https://github.com/ggml-org/llama.cpp) (MIT)
statically via [`llama-cpp-2`](https://crates.io/crates/llama-cpp-2); its model is
[**jina-embeddings-v2-base-code**](https://huggingface.co/jinaai/jina-embeddings-v2-base-code)
by [Jina AI](https://jina.ai/) (Apache-2.0), fetched on first use as a `Q4_K_M`
GGUF quantization (a derivative work under Apache-2.0 §4) from the
[`semantic-model-v1`](https://github.com/get-tmonier/argot/releases/tag/semantic-model-v1)
release. argot is not affiliated with, nor endorsed by, Jina AI.

## License

MIT
