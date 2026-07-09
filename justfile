VERSION := `grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2`

default: help

help:
    @just --list

# Build the release binary + install landing deps.
install:
    cargo build --release -p argot
    cd landing && bun install

# Build the single `argot` release binary → target/release/argot.
build:
    cargo build --release -p argot

# --- pipeline (single Rust binary) ---

extract path=".":
    cargo run --release -p argot -- extract --repo {{path}}

train path=".":
    cargo run --release -p argot -- train --repo {{path}}

calibrate path=".":
    cargo run --release -p argot -- calibrate --repo {{path}}

# Fit = train + calibrate in one shot.
fit path=".":
    cargo run --release -p argot -- fit --repo {{path}}

check path="." ref="HEAD~1..HEAD":
    cargo run --release -p argot -- check --repo {{path}} {{ref}}

# --- benchmarks (crates/argot-bench) ---

# Full recall/FP bench over every corpus in benchmarks/targets.yaml.
bench:
    cargo build --release -p argot-bench
    ./target/release/argot-bench --results-dir benchmarks/results/latest

# ~1 min smoke: one fixture per category + 50 controls on ink.
bench-quick:
    cargo build --release -p argot-bench
    ./target/release/argot-bench --corpus ink --quick --results-dir benchmarks/results/quick

# Semantic-layer bench (F1 reinvention + F2 placement: recall AND clean-commit FP)
# over every corpus with fixtures. Builds the semantic binary (feature is off in
# dev/CI, on only for shipped builds), then runs the unified driver. Pass corpora
# to scope (`just bench-semantic rich hono`); needs numpy (benchmarks/requirements.txt)
# and the jina GGUF (auto-downloaded, or ARGOT_SEMANTIC_MODEL=<path>). Robust: each
# fit is timeout+retry-guarded so one huge corpus (e.g. dagster, 14.8k fns) can't stall.
bench-semantic *corpora:
    cargo build --release -p argot --features semantic
    python3 benchmarks/sem_all.py {{corpora}}
    python3 benchmarks/sem_consolidate.py   # → landing/src/data/semantic.json

# Structural-foreignness floor validation over every corpus: real multi-language
# extraction + real temporal-holdout over-fire. Pure-Rust feature (no model/deps),
# NON-GATING and off in shipped builds — it exists to validate the irreducible
# floor (docs/research/evidence/foreign-structure-gate-floor.md), not to gate.
# Pass corpora to scope (`just bench-structural --corpus rich,faker`).
bench-structural *args:
    cargo build --release -p argot-bench --features structural
    ./target/release/argot-bench --mode structural --results-dir benchmarks/results/structural {{args}}

# --- checks ---

# Format check + clippy-as-errors + tests. Canonical CI gate.
verify:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    @echo "✓ all checks passed"

verify-fix:
    cargo fmt
    cargo clippy --workspace --all-targets --fix --allow-dirty -- -D warnings
    cargo test --workspace

test:
    cargo test --workspace

# Dev profile on purpose: after `just verify` (or CI's cargo test) the dev
# binary is already built, so smoke adds no rebuild.
smoke:
    cargo run -p argot -- extract --repo . && test -s .argot/dataset.jsonl

ci: verify smoke

# Run the full pipeline against a path (default: argot itself) and assert the
# outputs are shaped — both .py and .ts rows in dataset.jsonl + scorer-config
# emitted. Dev-loop signal that monorepo handling didn't silently break.
dogfood path=".":
    cargo build --release -p argot
    ./target/release/argot extract --repo {{path}}
    ./target/release/argot train --repo {{path}}
    ./target/release/argot calibrate --repo {{path}}
    ./target/release/argot check --repo {{path}} || true
    test -s .argot/dataset.jsonl || (echo "✗ dataset.jsonl empty/missing" && exit 1)
    grep -qE '"file_path": "[^"]*\.py"' .argot/dataset.jsonl || (echo "✗ no .py rows" && exit 1)
    grep -qE '"file_path": "[^"]*\.tsx?"' .argot/dataset.jsonl || (echo "✗ no .ts rows" && exit 1)
    test -s .argot/scorer-config.json || (echo "✗ scorer-config.json missing" && exit 1)
    @echo "✓ dogfood: pipeline ran end-to-end, both .py and .ts rows, scorer-config emitted"

# --- landing site (argot.tmonier.com) · standalone project, own deps ---

landing:
    cd landing && bun install && bun run dev

landing-check:
    cd landing && bun run check

landing-build:
    cd landing && bun run build

# --- release ---
# Releases are cut by tagging `v<x.y.z>`; the `release` workflow (cargo-dist)
# builds the cross-platform binaries, attaches them to the GitHub Release, and
# publishes the `@tmonier/argot` npm wrapper that downloads the right binary.
