export ARGOT_DEV := "1"
REPO_ROOT := "/Users/damienmeur/projects/argot"

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

# --- research ---

research-concat out +inputs:
    uv run --package argot-engine python -m argot.corpus concat {{inputs}} -o {{out}}

research-benchmark dataset=".argot/research/combined.jsonl" sizes="500,2000,8000" seeds="3":
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset {{dataset}} --sizes {{sizes}} --seeds {{seeds}} \
        --out .argot/research/results.jsonl

research-benchmark-bpe:
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset {{REPO_ROOT}}/.argot/research/buckets/small.jsonl --sizes 3000 --seeds 3 \
        --encoder bpe --out {{REPO_ROOT}}/.argot/research/results-bpe.jsonl
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset {{REPO_ROOT}}/.argot/research/buckets/medium.jsonl --sizes 7000 --seeds 3 \
        --encoder bpe --out {{REPO_ROOT}}/.argot/research/results-bpe.jsonl
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset {{REPO_ROOT}}/.argot/research/buckets/large.jsonl --sizes 20000 --seeds 3 \
        --encoder bpe --out {{REPO_ROOT}}/.argot/research/results-bpe.jsonl

research-benchmark-token-embed:
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset {{REPO_ROOT}}/.argot/research/buckets/small.jsonl --sizes 3000 --seeds 3 \
        --encoder token_embed --out {{REPO_ROOT}}/.argot/research/results-token-embed.jsonl
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset {{REPO_ROOT}}/.argot/research/buckets/medium.jsonl --sizes 7000 --seeds 3 \
        --encoder token_embed --out {{REPO_ROOT}}/.argot/research/results-token-embed.jsonl
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset {{REPO_ROOT}}/.argot/research/buckets/large.jsonl --sizes 20000 --seeds 3 \
        --encoder token_embed --out {{REPO_ROOT}}/.argot/research/results-token-embed.jsonl

research-benchmark-combined seeds="3" out=".argot/research/results-combined.jsonl":
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets/small.jsonl --sizes 3000 --seeds {{seeds}} --out {{out}}
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets/medium.jsonl --sizes 7000 --seeds {{seeds}} --out {{out}}
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets/large.jsonl --sizes 20000 --seeds {{seeds}} --out {{out}}

research-extract-honest:
	scripts/research/extract-honest-corpus.sh

research-concat-honest:
	uv run --package argot-engine python -m argot.corpus concat \
		.argot/research/datasets-v2/httpx.jsonl .argot/research/datasets-v2/requests.jsonl \
		-o .argot/research/buckets-v2/small-py.jsonl
	uv run --package argot-engine python -m argot.corpus concat \
		.argot/research/datasets-v2/ky.jsonl .argot/research/datasets-v2/zod.jsonl \
		-o .argot/research/buckets-v2/small-ts.jsonl
	uv run --package argot-engine python -m argot.corpus concat \
		.argot/research/datasets-v2/fastapi.jsonl .argot/research/datasets-v2/flask.jsonl \
		-o .argot/research/buckets-v2/medium-py.jsonl
	uv run --package argot-engine python -m argot.corpus concat \
		.argot/research/datasets-v2/vite.jsonl .argot/research/datasets-v2/typescript-eslint.jsonl \
		-o .argot/research/buckets-v2/medium-ts.jsonl
	uv run --package argot-engine python -m argot.corpus concat \
		.argot/research/datasets-v2/pydantic.jsonl .argot/research/datasets-v2/django.jsonl \
		-o .argot/research/buckets-v2/large-py.jsonl
	uv run --package argot-engine python -m argot.corpus concat \
		.argot/research/datasets-v2/effect.jsonl .argot/research/datasets-v2/angular.jsonl \
		-o .argot/research/buckets-v2/large-ts.jsonl

research-honest-benchmark encoder="tfidf" seeds="3" out=".argot/research/results-honest.jsonl":
	uv run --package argot-engine python -m argot.corpus benchmark \
		--dataset .argot/research/buckets-v2/small-py.jsonl --sizes 3000 --seeds {{seeds}} \
		--encoder {{encoder}} --out {{out}}
	uv run --package argot-engine python -m argot.corpus benchmark \
		--dataset .argot/research/buckets-v2/small-ts.jsonl --sizes 3000 --seeds {{seeds}} \
		--encoder {{encoder}} --out {{out}}
	uv run --package argot-engine python -m argot.corpus benchmark \
		--dataset .argot/research/buckets-v2/medium-py.jsonl --sizes 7000 --seeds {{seeds}} \
		--encoder {{encoder}} --out {{out}}
	uv run --package argot-engine python -m argot.corpus benchmark \
		--dataset .argot/research/buckets-v2/medium-ts.jsonl --sizes 7000 --seeds {{seeds}} \
		--encoder {{encoder}} --out {{out}}
	uv run --package argot-engine python -m argot.corpus benchmark \
		--dataset .argot/research/buckets-v2/large-py.jsonl --sizes 20000 --seeds {{seeds}} \
		--encoder {{encoder}} --out {{out}}
	uv run --package argot-engine python -m argot.corpus benchmark \
		--dataset .argot/research/buckets-v2/large-ts.jsonl --sizes 20000 --seeds {{seeds}} \
		--encoder {{encoder}} --out {{out}}

acceptance entry="":
	uv run --package argot-engine python -m argot.acceptance.runner \
		--catalog engine/argot/acceptance/catalog \
		--out docs/research/scoring/acceptance \
		--epochs 20 $([ -n "{{entry}}" ] && echo "--entry {{entry}}")

phase12-s0:
    uv run --package argot-engine python -m argot.research.signal.cli.seeded_ci \
        --out docs/research/scoring/signal/phase12

phase12-s2:
    uv run --package argot-engine python -m argot.research.signal.cli.bakeoff \
        --scorers delta_mlm_mean,delta_mlm_min,delta_mlm_p05 \
        --context-mode file_only \
        --entry fastapi \
        --out docs/research/scoring/signal/phase12

phase12-s3:
    uv run --package argot-engine python -m argot.research.signal.cli.bakeoff \
        --scorers refactor_contrastive \
        --context-mode file_only \
        --entry fastapi \
        --out docs/research/scoring/signal/phase12

phase12-s1s4:
    uv run --package argot-engine python -m argot.research.signal.cli.bakeoff \
        --scorers mlm_surprise_mean,mlm_surprise_min,mlm_surprise_p05,tfidf_anomaly,knn_cosine,lof_embedding,lm_perplexity,ast_structural_ll,ast_structural_zscore,ast_structural_oov \
        --context-mode file_only \
        --entry fastapi \
        --out docs/research/scoring/signal/phase12

phase11:
    uv run --package argot-engine python -m argot.research.signal.sweep \
        --stage 5 --entry fastapi --configs mean_z --context-mode baseline \
        --out docs/research/scoring/signal/phase11
    uv run --package argot-engine python -m argot.research.signal.sweep \
        --stage 5 --entry fastapi --configs mean_z --context-mode parent_only \
        --out docs/research/scoring/signal/phase11
    uv run --package argot-engine python -m argot.research.signal.sweep \
        --stage 5 --entry fastapi --configs mean_z --context-mode file_only \
        --out docs/research/scoring/signal/phase11
    uv run --package argot-engine python -m argot.research.signal.sweep \
        --stage 5 --entry fastapi --configs mean_z --context-mode siblings_only \
        --out docs/research/scoring/signal/phase11
    uv run --package argot-engine python -m argot.research.signal.sweep \
        --stage 5 --entry fastapi --configs mean_z --context-mode combined \
        --out docs/research/scoring/signal/phase11

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
