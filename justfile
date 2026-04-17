default: help

help:
    @just --list

install:
    bun install && uv sync

extract path=".":
    uv run --package argot-engine python -m argot.extract {{path}}

lint:
    bun run lint && uv run ruff check engine

format:
    bun run format && uv run ruff format engine

typecheck:
    bun run typecheck && uv run mypy engine

boundaries:
    bun run boundaries

test:
    bun test && uv run pytest engine

ci: lint typecheck boundaries test smoke

smoke:
    just extract . && test -s .argot/dataset.jsonl

bump:
    ncu -u && bun install && uv lock --upgrade
