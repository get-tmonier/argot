# argot

Voice linter that learns a repo's voice from git history. CLI in TypeScript/Bun; data pipeline in Python/UV.

## Guiding principle

**In doubt, optimise for code that's easy to change.** The Pragmatic Programmer / craftsmanship lens: the right design is the one a future contributor (human or agent) can extend, refactor, or revert without archaeology. When two options look equally correct, pick the one with the smaller blast radius and clearer seams. Don't add abstractions before the second use case shows up; don't keep dead code "just in case"; don't suppress a check when the underlying code is the real fix. Strict tooling (mypy, no-any, ruff, dependency-cruiser) exists to surface change-cost early — work with it, not around it.

## Task runner

Always use `just` — it's the canonical interface for all dev commands.

```
just verify       # full check suite (lint + format + typecheck + boundaries + knip + test)
just test         # bun test --cwd cli && uv run pytest engine
just extract .    # run extract pipeline on this repo → .argot/dataset.jsonl
just dogfood      # run full pipeline against argot itself (or any path) — fast monorepo check
just install      # bun install + uv sync
```

`just dogfood` exercises extract → train → calibrate → check end-to-end and asserts both Python and TypeScript rows landed in `dataset.jsonl` plus a `scorer-config.json` was emitted. It's a **dev loop, not a CI gate** — informational signal that monorepo handling didn't silently break. Drift is the contributor's responsibility; nothing forces it to run.

## Architecture

**CLI** (`cli/src/`) uses hexagonal architecture, enforced by dependency-cruiser:

```
modules/<name>/
  domain/           # pure types — no imports from other layers
  application/      # use-cases + outbound port interfaces (ports/out/)
  infrastructure/   # adapters implementing ports (may do I/O)
  dependencies.ts   # Effect Layer composition for this module
shell/              # inbound CLI commands — wires Layers, no module infra imports
dependencies.ts     # root Layer composition
```

**Engine** (`engine/argot/`) is a Python subprocess. The CLI's `BunEngineRunner` adapter spawns `uv run argot-engine extract`. It outputs JSONL to `.argot/dataset.jsonl`. The full pipeline is: `argot-extract` → `argot-train` → `argot-calibrate` → `argot-check`.

`engine/argot/` must not import from experimental research branches; production code lives under `engine/argot/scoring/`. Production symbols (classes, files, functions) must be named after domain concepts — never after research artefacts (`era`, `phase`, `PhaseNa…`, etc.); those labels belong in bench/research code only.

## Key conventions

- All side-effects in CLI go through Effect — `Console.log` not `console.log`
- Cross-module imports only via `<module>/dependencies.ts`, never into inner layers
- Path aliases `#modules/*`, `#shell/*`, `#dependencies` — no relative `../..` across layers
- TypeScript strict + `no-any`; Python mypy strict + ruff (line length 100)
- Test files: `*.test.ts` (Bun), `test_*.py` (pytest)

## Testing

Write tests alongside any new logic — not 100% coverage, but enough for a fast feedback loop. Aim to cover:
- Core logic correctness (shapes, invariants, non-trivial conditions)
- Smoke tests for new entry points

For non-trivial production logic (scoring math, threshold decisions, cluster logic), write unit tests that test behaviour, not implementation: assert on outputs for given inputs, not on internal state or call sequences. Tests should survive a refactor that preserves semantics.

## Language and corpus independence

Production code (`engine/argot/scoring/`, `cli/src/`) must be language-agnostic and corpus-agnostic. No hardcoded references to Python, TypeScript, FastAPI, faker-js, or any other specific language or corpus. Those appear only in fixtures, benchmarks, and eval scripts. A scorer that only works on Python repos is not a production scorer.

## Code quality

The codebase is strict by design (mypy strict, no-any, ruff). When a check fails:
- Diagnose the exact root cause before fixing
- Prefer targeted fixes (`# type: ignore[specific-code]` on one line) over global config changes
- Never add broad suppressions (`ignore_missing_imports = true` globally, etc.) to make errors go away

No abusive lint shortcuts in production code (`engine/argot/` outside scripts, `cli/src/`):
- No file- or module-wide disables: `# ruff: noqa`, `# mypy: ignore-errors`, `/* eslint-disable */`, `// oxlint-disable-file`, `// @ts-nocheck`, etc.
- No blanket per-line disables either: `# noqa` without a rule code, `// oxlint-disable-next-line` without a rule name. Always cite the specific rule.
- Targeted single-line ignores with a specific rule code (`# type: ignore[arg-type]`, `// oxlint-disable-next-line no-explicit-any`) are fine when the lint is genuinely wrong about a specific case — explain why in a one-line comment.
- Exception: `benchmarks/` and `engine/argot/scripts/` may use file-level disables. They're throwaway research code where signal-over-cleanliness is the right tradeoff.

We aim for clean architecture and clean code; lint-suppression debt compounds and is the wrong knob to turn when a check fails. The right knob is the underlying code.

## Toolchain (managed by mise)

`bun 1.3.12` · `python 3.13` · `uv 0.11.7` · `just 1.49.0` · `lefthook 2.1.6`

Linting/checking: `oxlint` · `oxfmt` · `tsgo` (native TS checker) · `dependency-cruiser` · `knip` · `ruff` · `mypy`

## Research workflow

Benchmarks are expensive. Default to the cheapest signal first:

1. **Dirty experiment script** in `benchmarks/` — quick, ugly code is fine; what matters is the number, not the code.
2. **Scoped bench run** on one or two corpora — enough to confirm or kill a hypothesis.
3. **Full corpus bench** — final confirmation of a strong signal, or era-closing baseline. Not a default step.

Keep evidence of every experiment in `docs/research/evidence/` regardless of outcome. Clean up experiment scripts once results are recorded — they don't need to survive, the evidence does.

## Agent skills

### Issue tracker

Issues live as local markdown files under `.scratch/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Four-role vocabulary for solo maintainer (no `needs-info`). See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context layout; `docs/research/` serves as ADR. See `docs/agents/domain.md`.
