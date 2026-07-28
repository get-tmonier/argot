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
# Corpora run concurrently (`--jobs`, default min(cores, 8)) — each corpus is
# an independent fit → replay, and results are order-stable at any job count.
bench *args:
    cargo build --release -p argot-bench
    ./target/release/argot-bench --results-dir benchmarks/results/latest {{args}}

# ~1 min smoke: one fixture per category + 50 controls on ink.
bench-quick:
    cargo build --release -p argot-bench
    ./target/release/argot-bench --corpus ink --quick --results-dir benchmarks/results/quick

# Semantic-layer bench (F1 reinvention + F2 placement: recall AND clean-commit FP)
# over every corpus with fixtures. Builds the semantic binary (feature is off in
# dev/CI, on only for shipped builds), then runs the unified driver. Pass corpora
# to scope (`just bench-semantic rich hono`); needs numpy + PyYAML
# (benchmarks/requirements.txt) and the jina GGUF (auto-downloaded, or
# ARGOT_SEMANTIC_MODEL=<path>). Robust: each fit is timeout+retry-guarded so one
# huge corpus (e.g. dagster, 14.8k fns) can't stall.
#
# Runs SEM_JOBS corpora concurrently (default 4), each argot capped to
# ARGOT_THREADS=cpu//jobs so the fits divide the CPU instead of oversubscribing.
# Verified iso: the embed cache is content-addressed and thread-count-invariant,
# so parallelism only trades wall-clock, never numbers (measured 73m→25.5m, 2.86x,
# byte-identical results). `SEM_JOBS=1 just bench-semantic` forces sequential.
bench-semantic *corpora:
    cargo build --release -p argot --features semantic
    python3 benchmarks/sem_all.py --jobs ${SEM_JOBS:-4} {{corpora}}
    python3 benchmarks/sem_consolidate.py   # → landing/src/data/semantic.json

# Structural-foreignness floor validation over every corpus: real multi-language
# extraction + real temporal-holdout over-fire. Pure-Rust feature (no model/deps),
# NON-GATING and off in shipped builds — it exists to validate the irreducible
# floor (docs/research/evidence/foreign-structure-gate-floor.md), not to gate.
# Pass corpora to scope (`just bench-structural --corpus rich,faker`).
bench-structural *args:
    cargo build --release -p argot-bench --features structural
    ./target/release/argot-bench --mode structural --results-dir benchmarks/results/structural {{args}}

# Architecture-graph bench (`--features arch`): real recall on authored 0-usage
# violation fixtures + real-holdout over-fire, across the 23 corpora with
# meaningful layering. 11 languages; every corpus ≥88% real recall / 0% control-FP
# (docs/research/evidence/architecture-graph-foreignness.md). Feature-gated,
# NON-GATING in dev/CI, base guardrail byte-for-byte unchanged. Pass corpora to
# scope (`just bench-arch --corpus guava,ripgrep`); default runs the full set.
bench-arch *args:
    cargo build --release -p argot-bench --features arch
    ./target/release/argot-bench --mode arch --results-dir benchmarks/results/arch \
      {{ if args == "" { "--corpus saleor,scrapy,wagtail,fastapi,faker,dagster,composer,laravel,ripgrep,bat,guava,junit5,powershell,jellyfin,rubocop,gh-cli,hugo,hono,eslint,excalidraw,faker-js,curl,rocksdb,mormot2,castle-engine" } else { args } }}

# Fast fixture-recall guard (`--mode arch-verify`): fit each corpus at HEAD and
# score its authored fixtures, skipping the slow holdout replay (~25s for all 23
# corpora vs ~12min). Use as a regression check when the resolver changes — any
# `invalid` count or recall drop means fixtures rotted. Full over-fire is in
# `just bench-arch`.
arch-verify *args:
    cargo build --release -p argot-bench --features arch
    ./target/release/argot-bench --mode arch-verify \
      {{ if args == "" { "--corpus saleor,scrapy,wagtail,fastapi,faker,dagster,composer,laravel,ripgrep,bat,guava,junit5,powershell,jellyfin,rubocop,gh-cli,hugo,hono,eslint,excalidraw,faker-js,curl,rocksdb,mormot2,castle-engine" } else { args } }}

# Dump the resolver-verified 0-usage candidate menu (`--mode arch-candidates`) —
# ready-to-author fixture rows (host_file + verified import_line) per corpus.
bench-arch-candidates *args:
    cargo build --release -p argot-bench --features arch
    ./target/release/argot-bench --mode arch-candidates --results-dir benchmarks/results/arch {{args}}

# Fit each corpus at its pinned SHA on the production path, apply every
# authored test-gaming fixture as a real staged edit, judge with
# `check --staged` (authored controls must stay silent).
# Gaming-fixture recall guard (`--mode integrity-verify`).
integrity-verify *args:
    cargo build --release -p argot-bench --features integrity
    ./target/release/argot-bench --mode integrity-verify {{args}}

# Replay accepted test-touching commits OUTSIDE the fit's calibration window
# through the fitted gates (gate = ≤2% flagged).
# Accepted-history FP for the integrity rules (`--mode integrity-fp`).
bench-integrity-fp *args:
    cargo build --release -p argot-bench --features integrity
    ./target/release/argot-bench --mode integrity-fp {{args}}

# --- checks ---

# Format check + clippy-as-errors + tests. Canonical CI gate.
verify:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    @just verify-features
    @just _disk-guard
    @echo "✓ all checks passed"

# `target/*/incremental` is a pure rebuild cache — deleting it costs one slower
# recompile and nothing else — and it is the fastest-growing thing here: it
# reached 57 GB, more than the rest of debug/ put together.
# Reclaim disk from the build tree (safe any time; removes no build input).
clean-cache:
    @echo "before: $(du -sh target 2>/dev/null | cut -f1) in target/, $(df -h . | tail -1 | awk '{print $4}') free"
    rm -rf target/debug/incremental target/release/incremental target/tmp
    @echo "after:  $(du -sh target 2>/dev/null | cut -f1) in target/, $(df -h . | tail -1 | awk '{print $4}') free"

# Warn — never delete — when the rebuild cache has grown past what a laptop
# wants to carry. Cargo never garbage-collects target/, so it grows without
# bound. This measures only what `clean-cache` can actually reclaim: warning on
# total target/ size would keep firing after a clean, and an alarm you cannot
# act on is one you learn to ignore. Advisory by design — a build tree
# disappearing under a running agent is worse than a full disk.
_disk-guard:
    #!/usr/bin/env bash
    gb=$(du -sgc target/*/incremental 2>/dev/null | tail -1 | cut -f1)
    if [ "${gb:-0}" -ge 15 ]; then
      echo "⚠ ${gb} GB of rebuild cache in target/ — \`just clean-cache\` reclaims it"
    fi

# The feature-gated slices, which `verify`'s featureless base loop does not
# build. Release binaries ship every one of them, so a green base loop verifies
# a configuration nobody runs: a test asserting behaviour that had changed sat
# green locally through several pushes and only failed in CI. These three are
# pure Rust and cost seconds; `semantic` needs the llama.cpp C++ build and stays
# CI-only (see "Keep PR CI fast").
verify-features:
    for f in script arch integrity; do \
        cargo clippy --workspace --all-targets --features "$f" -- -D warnings || exit 1; \
        cargo test --workspace --features "$f" || exit 1; \
    done

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

# --- showcase: dogfood argot on real OSS repos (proof-post / demo assets) ---

# Run the SHIPPED `argot audit` (zero-setup, no fit) across popular repos →
# per-repo JSON + screenshot-ready HTML cards under {{out}}, then a summary.
# Needs `argot` on PATH (`just build`, then add target/release, or install it).
# Override the list: just audit-map "fastapi/fastapi cli/cli".
audit-map repos="fastapi/fastapi django/django pallets/flask encode/httpx vercel/next.js facebook/react vuejs/core withastro/astro golang/go cli/cli rust-lang/cargo" out="benchmarks/results/audit-map":
    #!/usr/bin/env bash
    set -euo pipefail
    command -v argot >/dev/null || { echo "✗ argot not on PATH — 'just build' then add target/release, or install argot"; exit 1; }
    mkdir -p "{{out}}"
    for slug in {{repos}}; do
      name="${slug//\//_}"
      echo "→ $slug"
      tmp="$(mktemp -d)"
      if git clone --quiet --depth 200 "https://github.com/$slug" "$tmp/repo" 2>/dev/null; then
        ( cd "$tmp/repo" && argot audit --commits 50 --format json ) > "{{out}}/$name.json" 2>/dev/null || echo "  audit(json) failed"
        ( cd "$tmp/repo" && argot audit --commits 50 --format html ) > "{{out}}/$name.html" 2>/dev/null || echo "  audit(html) failed"
      else
        echo "  clone failed — skipping"
      fi
      rm -rf "$tmp"
    done
    echo "── audit-map → {{out}}/ ──"
    for f in "{{out}}"/*.json; do
      [ -e "$f" ] || continue
      command -v jq >/dev/null && printf '%-26s commits=%s ai=%s findings=%s\n' \
        "$(basename "$f" .json)" "$(jq '.commits.total' "$f")" "$(jq '.commits.ai_assisted' "$f")" "$(jq '.findings | length' "$f")"
    done

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
