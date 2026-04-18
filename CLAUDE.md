# argot

Style linter that learns a repo's voice from git history. CLI in TypeScript/Bun; data pipeline in Python/UV.

## Task runner

Always use `just` — it's the canonical interface for all dev commands.

```
just verify       # full check suite (lint + format + typecheck + boundaries + knip + test)
just test         # bun test --cwd cli && uv run pytest engine
just extract .    # run extract pipeline on this repo → .argot/dataset.jsonl
just install      # bun install + uv sync
```

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

**Engine** (`engine/argot/`) is a Python subprocess. The CLI's `BunEngineRunner` adapter spawns `uv run argot-engine extract`. It outputs JSONL to `.argot/dataset.jsonl`.

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

## Code quality

The codebase is strict by design (mypy strict, no-any, ruff). When a check fails:
- Diagnose the exact root cause before fixing
- Prefer targeted fixes (`# type: ignore[specific-code]` on one line) over global config changes
- Never add broad suppressions (`ignore_missing_imports = true` globally, etc.) to make errors go away

## Toolchain (managed by mise)

`bun 1.3.11` · `node 22` · `python 3.11` · `uv 0.5.0` · `just 1.34.0` · `lefthook 1.7.0`

Linting/checking: `oxlint` · `oxfmt` · `tsgo` (native TS checker) · `dependency-cruiser` · `knip` · `ruff` · `mypy`

## Homebrew tap

Formula lives in a separate repo: https://github.com/tmonier/homebrew-argot
The `update-homebrew-tap` job in `.github/workflows/release.yml` auto-updates it on each release.
Requires secret `HOMEBREW_TAP_TOKEN` (PAT with `repo` scope on `tmonier/homebrew-argot`) in the main repo's GitHub Actions secrets.
