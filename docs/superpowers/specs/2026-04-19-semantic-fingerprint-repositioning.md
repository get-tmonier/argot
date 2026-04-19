# Semantic Fingerprint Repositioning — Design Spec

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Validate and ship the pivot from micro-syntactic style detection to semantic fingerprint detection — code that diverges from how a repo handles errors, validates data, composes logic, and manages side effects.

**Context:** Phase 7.3 results showed `injected_auc 0.94` and `cross_auc 0.73` with the pretrained CodeRankEmbed encoder, but `synthetic_auc_mean ~0.51` (near random) on micro-syntactic mutations (case_swap, quote_flip). This confirms the model is a semantic similarity detector, not a syntactic one. The repositioning capitalises on what actually works.

**Architecture:** Three sequential phases — spot-check to validate the signal exists on real repos, benchmark extension to quantify it, then product repositioning conditioned on confirmed results.

**OSS strategy:** Local-first, cloud-never for the core. BYOAI via `argot check --explain | <any-llm>` for natural language explanations. CI/LLM integration kept as a future commercial lever.

---

## Phase 1 — Spot-check (go/no-go gate)

**Goal:** Confirm the pretrained encoder scores semantically foreign injections higher than native code on two real repos before investing in benchmark infrastructure.

### Repos

**Repo 1 — argot CLI** (`cli/src/`)

Effect-ts consumer with a very strong semantic fingerprint:
- All effects via `Effect.gen` / `yield*` — no `async/await`
- Errors via `Effect.fail` — no `try/catch`, no `throw`
- Side effects via `Console.log` (Effect) — no `console.log`
- DI via `Layer` / `Effect.Service` — no `new Foo()`
- Validation via `Schema.parse` — no `if/else` guards
- Composition via `pipe()` — no nested calls

**Repo 2 — httpx** (Python, already extracted in benchmark)

Strong semantic fingerprint:
- `async/await` structured throughout
- Typed errors via `httpx.HTTPStatusError` hierarchy
- Exhaustive type hints on all functions
- Structured logging with named fields
- `pathlib.Path` for file handling
- Context managers (`with`) for resource management

### Injection categories

For each repo, 5–6 hand-crafted injections per category:

| # | category | native pattern | foreign injection |
|---|---|---|---|
| 1 | error handling | `Effect.fail` / typed exceptions | `try/catch` + `throw new Error()` / `except Exception: pass` |
| 2 | side effects | `Console.log` (Effect) / structured logger | `console.log` / `print()` |
| 3 | validation | `Schema.parse` / Pydantic | `if/else` manual guards |
| 4 | composition | `pipe(a, f, g)` / `Effect.gen` | `g(f(a))` nested / `async def` imperative |
| 5 | DI / instantiation | `Layer` / `Effect.Service` | `new Foo()` direct / module-level singleton |
| 6 | null/option handling | discriminated unions / `Option` | `null` checks + `!` assertions |

Additional cross-language patterns to consider:
- Typed config via env schema vs `process.env.FOO` direct
- `const` + immutability vs in-place mutation
- `map/filter/reduce` vs `for` loops
- Guard clauses / early return vs nested `if` pyramids
- Rich contextual error messages vs bare `"error occurred"` strings
- `dataclass` / `@property` vs raw mutable attributes (Python)
- `pathlib` vs `os.path` (Python)
- Generators / `yield` vs list accumulation (Python)

### Success criterion

**Delta ≥ 0.20** between mean injected score and mean native score across both repos.

If delta < 0.20: stop, the model doesn't see the difference at this granularity — reassess model or granularity before proceeding to Phase 2.

### Output

`docs/research/scoring/phase-8/spot-check.md` — raw scores per injection, per category, per repo, with the go/no-go conclusion.

---

## Phase 2 — Semantic mutation benchmark

**Goal:** Replace `synthetic_auc_mean` (which measured micro-syntactic mutations the model is blind to) with `semantic_auc_mean` as the primary success metric.

### New mutators

Add to `engine/argot/` alongside existing synthetic mutators:

| mutator | what it does |
|---|---|
| `semantic_error_pattern` | replace native error idiom (Result, Effect.fail, typed exception hierarchy) with try/catch + generic throw |
| `semantic_logging_pattern` | replace structured logger / Effect.log with console.log / print() |
| `semantic_validation_pattern` | replace Schema/Zod/Pydantic with manual if/else guards |
| `semantic_composition_pattern` | replace pipe() / Effect.gen / generators with nested calls / async-await imperative |
| `semantic_di_pattern` | replace Layer/Service/DI framework with direct instantiation |

### New metric

`semantic_auc_mean` = mean of the 5 semantic mutator AUCs.

Logged per mutator: `semantic_auc_error_pattern`, `semantic_auc_logging_pattern`, `semantic_auc_validation_pattern`, `semantic_auc_composition_pattern`, `semantic_auc_di_pattern`.

### Success criterion (Phase 7.4)

`semantic_auc_mean ≥ 0.75` at the **small** bucket, on ≥ 2 seeds out of 3.

Conservative relative to the original 0.85 target — we want a confirmed modest signal, not an unmet ambition.

### Legacy metrics

`synthetic_auc_mean` and its sub-metrics (case_swap, quote_flip, error_flip, debug_inject) remain logged for traceability but are removed from the primary success criteria.

---

## Phase 3 — Product repositioning

**Conditioned on Phase 2 passing.** If Phase 2 fails, document the negative result and reassess architecture before touching product docs.

### VISION.md changes

**Current pitch:** "style linter that learns a repo's voice from git history"

**New pitch:** "argot learns the semantic fingerprint of your codebase — how it handles errors, validates data, composes logic — and flags code that doesn't belong"

**What stays intact:**
- Local by default, cloud never (for core)
- Linter UX: exit codes, ranked list, no auto-fix
- TypeScript first, Python second
- One model per repo

**What changes:**
- Remove micro-syntactic detection from goals
- Reformulate primary use case: LLM-assisted contributions and semantically foreign copy-paste
- Add BYOAI explain mode as a v1 feature

### ROADMAP changes

| version | old criterion | new criterion |
|---|---|---|
| v0 | flag a stylistic outlier on Vigie | `semantic_auc_mean ≥ 0.75` confirmed on 2 repos; spot-check validated |
| v0.5 | used daily for 2 weeks | installed on argot itself; flags ≥ 1 real hunk in first 2 weeks |
| v1 | 3 external users | demo GIF shareable + GitHub Action template + 1 Show HN post |
| v2 | GitHub App | CI + LLM explain integration (commercial lever) |

### CLI output redesign

Three tiers of output, all generated locally:

**Tier 1 — score only (default)**
```
$ argot check HEAD~1

⚠  2 hunks diverge from this repo's semantic fingerprint

cli/src/modules/engine/infrastructure/runner.ts:47   0.94  ████████████████░░░░
cli/src/shell/commands/train.ts:82                   0.71  ██████████████░░░░░░
```

**Tier 2 — score + nearest neighbor (`--context`)**
```
$ argot check HEAD~1 --context

⚠  cli/src/modules/engine/infrastructure/runner.ts:47   score 0.94

  nearest matches in your codebase:
  → src/modules/corpus/infrastructure/extractor.ts:23  (similarity 0.91)
  → src/modules/engine/infrastructure/validator.ts:41  (similarity 0.87)
```

The nearest neighbor is a natural output of the KnnHead already implemented — requires storing file path + line metadata alongside embeddings at training time.

**Tier 3 — score + nearest neighbor + LLM explanation (BYOAI)**
```
$ argot check HEAD~1 --explain | claude
$ argot check HEAD~1 --explain | llm  # simonw/llm
$ argot check HEAD~1 --explain | ollama run mistral
```

`--explain` outputs a structured prompt (flagged hunk + nearest neighbors + repo context) to stdout. The user pipes it to any LLM. argot has zero dependency on any AI provider. Decoupled by design.

### Demo GIF spec

**Repo:** argot CLI itself (self-hosting is a strong signal)

**Setup:** a branch with 3 commits, one containing LLM-generated code using async/await + try/catch in an Effect codebase.

**Sequence:**
1. `argot train` — 8 seconds, ends with "learned N patterns from M commits"
2. `argot check HEAD~1` — 2 seconds, shows the two flagged hunks with scores
3. `argot check HEAD~1 --context` — shows nearest neighbors
4. `argot check HEAD~1 --explain | claude` — shows LLM explanation appearing

Total GIF length: ~20 seconds. No voiceover needed — the output speaks for itself.

---

## Non-goals (explicit)

- Micro-syntactic detection (quote style, casing, line length) — not in scope, not detected by the model
- Cloud-hosted analysis — never
- Auto-fixing — argot flags, the human decides
- Cross-repo learning in v1 — one model per repo
- LLM integration as a hard dependency — always optional, always BYOAI
