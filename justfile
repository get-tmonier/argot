export ARGOT_DEV := "1"

VERSION := `bun -e "console.log(require('./cli/package.json').version)"`

default: help

help:
    @just --list

install:
    bun install && uv sync

build:
    mkdir -p dist
    cd cli && bun build --compile --target=bun src/cli.ts \
        --define "ARGOT_VERSION=\"$(VERSION)\"" \
        --outfile ../dist/argot

extract path=".":
    uv run --package argot-engine python -m argot.extract {{path}}

train dataset=".argot/dataset.jsonl" model=".argot/model.pkl":
    uv run --package argot-engine python -m argot.train --dataset {{dataset}} --out {{model}}

check ref="HEAD~1..HEAD" model=".argot/model.pkl":
    uv run --package argot-engine python -m argot.check . {{ref}} --model {{model}}

fetch-training-data:
    uv run --package argot-engine python -m argot.fetch

poc-validate dataset=".argot/training.jsonl":
    uv run --package argot-engine python -m argot.validate --dataset {{dataset}}

benchmark model=".argot/model.pkl" dataset=".argot/dataset.jsonl" threshold="0.5":
    uv run --package argot-engine python -m argot.benchmark --model {{model}} --dataset {{dataset}} --threshold {{threshold}}

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

# --- release ---

publish-engine:
    cd engine && uv build && uv publish

release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$(git branch --show-current)" != "main" ]; then
        echo "Error: must be on main branch to release" >&2
        exit 1
    fi
    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: working tree is dirty" >&2
        exit 1
    fi
    # Bump versions
    bun -e "
        const fs = require('fs');
        const p = JSON.parse(fs.readFileSync('cli/package.json', 'utf8'));
        p.version = '{{VERSION}}';
        fs.writeFileSync('cli/package.json', JSON.stringify(p, null, 2) + '\n');
    "
    sed -i '' 's/^version = .*/version = "{{VERSION}}"/' engine/pyproject.toml
    # Commit, tag, push
    git add cli/package.json engine/pyproject.toml
    git commit -m "chore: release v{{VERSION}}"
    git tag "v{{VERSION}}"
    git push origin main "v{{VERSION}}"
    echo "Released v{{VERSION}} — CI will build binaries and publish to PyPI"
