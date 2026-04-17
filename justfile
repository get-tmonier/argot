default: help

help:
    @just --list

install:
    bun install && uv sync

extract path=".":
    uv run --package argot-engine python -m argot.extract {{path}}

train dataset=".argot/dataset.jsonl" model=".argot/model.pkl":
    uv run --package argot-engine python -m argot.train --dataset {{dataset}} --out {{model}}

check ref="HEAD~1..HEAD" model=".argot/model.pkl":
    uv run --package argot-engine python -m argot.check . {{ref}} --model {{model}}

# --- individual checks ---

lint:
    bun run lint && uv run ruff check engine

lint-fix:
    bun run lint -- --fix && uv run ruff check --fix engine

format:
    bun run format && uv run ruff format --check engine

format-fix:
    bun run format && uv run ruff format engine

typecheck:
    bun run typecheck && uv run mypy engine

boundaries:
    bun run boundaries

knip:
    bun run knip

test:
    bun test --cwd cli && uv run pytest engine

smoke:
    just extract . && test -s .argot/dataset.jsonl

# --- combined ---

verify: lint format typecheck boundaries knip test
    @echo "✓ all checks passed"

verify-fix: lint-fix format-fix typecheck boundaries knip test
    @echo "✓ all checks passed (auto-fixes applied)"

ci: verify smoke

bump:
    ncu -u && bun install && uv lock --upgrade
