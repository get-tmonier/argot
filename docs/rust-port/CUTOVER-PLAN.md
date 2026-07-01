# Rust cutover / merge-readiness plan

Status: the port itself is **done, parity-proven, and faster** (see `PORTING-NOTES.md`
and `docs/research/evidence/rust-port-auc-parity.md`). The single `argot` binary
(`crates/argot-{core,cli}`) reproduces the Python engine byte-for-byte on all six
bench corpora (AUC, `bpe_score`, threshold, recall identical; fp identical-or-better)
and runs 3.5–23× faster on the shipped commands. Committed + pushed on
`feat/rust-rewrite`.

What remains is **not engine work** — it's shipping, CI, cutover mechanics, docs,
and marketing. This doc is the map.

## The cutover is smaller than it looks

`@tmonier/argot` (npm) is **already** a thin wrapper: `postinstall.js` downloads a
prebuilt binary named `argot-<target>` from the latest GitHub Release and
`bin/argot` execs it. Today that binary is Bun-compiled and it spawns the Python
engine via `uvx argot-engine`. The cutover is therefore mostly:

- swap the **downloaded binary** (Bun → Rust),
- **drop** the PyPI `argot-engine` publish and the `uvx` subprocess hop,
- keep the exact install UX (`npm i -g @tmonier/argot`, the curl installer, the
  release-asset naming `argot-<target>`).

No new distribution channel is required. The Rust binary is self-contained (embeds
the tokenizer + BPE baseline via `include_bytes!`), so there's no engine download
and no runtime (no Python, no uv, no Node beyond the optional npm shim).

---

## P0 — Correctness / release blockers

### 1. Cross-platform (Linux) determinism verification  ← the one real risk
Parity was verified on macOS/darwin only. CI + releases run on Linux. The
"bit-identical" claim rests on f64 arithmetic (BPE log-ratio sums, KMeans),
tokenizers, tree-sitter (C), and git2/libgit2 producing identical output on Linux.
IEEE-754 f64 with the same op order is deterministic across platforms and Rust
does not auto-fuse FMA, so this *should* hold — but it must be **verified**, not
assumed. Action: run the golden tests + a 1–2 corpus bench parity check on Linux
(CI runner or a container) before publishing any "identical" marketing claim.

### 2. Portable static build
- `git2` currently uses default features → links `openssl-sys`, `libssh2-sys`,
  `libz-sys` (system C libs). argot only does **local** git operations (no network),
  so switch to `git2 = { default-features = false, features = ["vendored-libgit2"] }`.
  Drops openssl/ssh entirely → clean static build. Re-run golden diff tests after
  (local diffs don't use ssh/https, so output stays identical).
- `tokenizers` uses `onig` (Oniguruma, C) and tree-sitter grammars compile C. These
  cross-compile fine with a C toolchain; use **cargo-zigbuild** (zig as the C
  cross-compiler) or per-target native runners.
- Target matrix (recommend): `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`
  (fully static), `x86_64-apple-darwin`, `aarch64-apple-darwin`, optionally
  `x86_64-pc-windows-msvc`. Today the release only builds linux-x64 + darwin-arm64.

### 3. Release pipeline rewrite (`.github/workflows/release.yml`)
- Replace the `build-binaries` job's `bun build --compile` with cargo cross-builds
  producing assets named `argot-<target>` (must match `npm/scripts/postinstall.js`).
  Recommend **cargo-dist** — it generates the build matrix, GitHub Release, shell +
  PowerShell installers, a Homebrew tap, and an npm shim, from one config.
- **Delete** the `publish-engine` (PyPI/uv trusted-publishing) job.
- Keep `publish-npm` (`@tmonier/argot`) and `create-release` (attach `argot-*`).
- Extend `postinstall.js` beyond `linux-x64`/`darwin-arm64` to match the new matrix
  (it currently `throw`s on anything else).

### 4. Version source-of-truth
Today `auto-release.yml` bumps `cli/package.json` + `engine/pyproject.toml` on every
`main` merge, tags `v<x>`, and `release.yml` sets `npm/package.json` at publish time.
Post-cutover:
- Make `Cargo.toml` `[workspace.package].version` the source of truth (currently
  `0.2.42`, already aligned).
- Update `auto-release.yml` to bump `Cargo.toml` (+ `Cargo.lock`, + `npm/package.json`)
  and stop touching `engine/pyproject.toml` / `cli/package.json` (or keep during the
  transition if the Python engine still ships).
- Fix the `npm/package.json` version drift (it's `0.1.0` while everything else is
  `0.2.42`).

### 5. Fix `Cargo.toml` `repository`
It reads `https://github.com/argot-lint/argot`; the real remote is
`github.com/get-tmonier/argot`. Fix before any `crates.io` publish.

---

## P1 — CI + cutover mechanics

### 6. Rust CI (`.github/workflows/ci.yml`)
Add a job running `just verify-rust` (`cargo fmt --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace`) on Linux **and** macOS (the
macOS run doubles as the cross-platform parity check from P0). The recipe already
exists.

### 7. Retire the TS CLI (`cli/src`)
The Rust CLI is a full replacement (extract/train/calibrate/fit/check/status/list/
update). Once cut over, delete `cli/`, and remove/scope its lint stack: dependency-
cruiser (`.dependency-cruiser.cjs`), knip (`knip.config.ts`), oxlint (`.oxlintrc.json`),
oxfmt (`.oxfmtrc.json`), `tsconfig*`, the tsgo typecheck, the `ts` CI job, the
`cli/**` lefthook hooks, and the root `package.json` workspace entry. (Bun/Node stay
only for `landing/`.)

### 8. Python engine (`engine/argot`) — keep or delete
Deleting it is clean **except the bench depends on it** (see #9). Options:
- **(a) Keep as a dev-only dependency** for the bench during the transition, drop it
  from the shipped release (already true — the Rust binary doesn't use it). Retire
  later once the bench is Rust-only.
- **(b) Delete now** and rewire the bench (#9) in the same PR.
Recommendation: (a) — smaller, reversible, keeps the parity oracle around.

### 9. Benchmark harness (`benchmarks/`, ~2.6k LOC Python) — the coupling
It imports the Python engine directly (`from argot.scoring...` in `score.py`/`run.py`)
for the default path **and** for the auto-select probe (`_probe_keep_cluster_rare_rule`
builds a Python `SequentialImportBpeScorer`) — so even `ARGOT_BENCH_RUST=1` still
needs the Python engine for that probe. To make the bench engine-independent:
- port the auto-select probe to a Rust `argot score --probe` mode (or have the Rust
  `score` command return `rare_branch_hunks_fired`), and
- point `_BPE_GENERIC_BASELINE` at the embedded baseline (or a `argot dump-baseline`).
Until then, the bench keeps `engine/argot` as a dev dependency (option 8a). The bench
is dev/research tooling — it never ships — so this is not a release blocker, just a
"can we finally delete Python" gate.

### 10. `justfile`
Re-point the canonical recipes (`extract`, `train`, `check`, `fit`, `verify`, `test`,
`dogfood`, `smoke`) at the Rust binary, or make `verify-rust`/`dogfood-rust` the
defaults and demote the Python/TS ones. Drop the ruff/mypy/pytest and
oxlint/oxfmt/tsgo/dep-cruiser/knip steps once #7/#8 land.

### 11. `.mise.toml`
Remove `python` and `uv` once the bench no longer needs them (#9); keep `bun` (landing),
`just`, `lefthook`. Rust is pinned separately in `rust-toolchain.toml`.

---

## P2 — Docs (accuracy)

Rewrite everything that describes the old stack or install path:
- **`README.md`** — badges (drop bun/python; add rust), install (drop the uv step and
  the "installs uv" note), the "Stack" line (single Rust binary), dev setup
  (`mise install` list, `just install`), repo-layout diagram (→ `crates/`). Add a
  performance section (table below).
- **`install.sh`** — delete the `uv` check (dead), fix the `argot train` reference,
  drop "~2GB download" (baseline is embedded).
- **`npm/README.md`** — remove the "requires uv on PATH" note (now zero deps).
- **`CLAUDE.md`**, **`CONTEXT-MAP.md`**, **`docs/agents/domain.md`** — replace the
  TS-CLI-+-Python-subprocess architecture with the single-Rust-binary model. (Keep a
  short "Python engine + bench" note while the transition dependency exists.)
- **`landing/src/content/docs/getting-started.md`** — remove the uv / "Python
  subprocess engine" install notes; the `npm i -g @tmonier/argot` and curl paths stay.

Unchanged / still accurate: language-support tables, the scoring-model explanation,
the feature list.

---

## P3 — Selling the rewrite (marketing)

The story: **same results, proven byte-for-byte; a single static binary; no Python /
Node / uv; no model download; multiples faster.**

- **CHANGELOG / release notes**: add a headline "Rewritten in Rust" entry with the
  numbers + the parity guarantee (the fact that it's *rigorously verified identical*,
  not rewrite-and-pray, is itself the strongest selling point).
- **Landing** (`argot.tmonier.com`, Astro): hero + a "how it works / benchmarks"
  section with the speed table and "single binary, zero runtime deps"; refresh the
  install snippet.
- **README**: a compact performance table + a "parity-verified" badge/line.

| command | Rust | Python | speedup |
|---|---|---|---|
| `extract` | 2.95s | 15.2s | 5.2× |
| `calibrate` | 2.2s | 7.8s | 3.5× |
| `check` | 0.015s | 0.345s | ~23× |
| full 6-corpus bench | 553s | 803s | 1.45× |

(Binary ~4.6 MB of source incl. embedded 4 MB tokenizer; compiled release binary is a
single file. Note the bench-wall figure is IPC-bound in the harness, not the engine —
lead with the per-command numbers.)

---

## Open decisions (need your call)

1. **Retire Python/TS now or after a transition release?** (Recommend: delete `cli/src`
   now — Rust CLI fully replaces it; keep `engine/argot` as a dev-only bench dep until
   the bench is Rust-only.)
2. **Platform matrix** — just linux-x64 + darwin-arm64 (today), or add darwin-x64 /
   linux-arm64 / musl-static / Windows?
3. **Extra install channels** — `cargo install argot` (crates.io) and/or a Homebrew
   tap, or keep npm-wrapper + curl only?
4. **Release tooling** — adopt cargo-dist (recommended; generates matrix + installers +
   npm shim + brew), or hand-roll a cargo-zigbuild matrix in the existing workflow?
5. **Bench** — port the harness (incl. auto-select probe) to Rust-only now, or keep the
   Python dev dependency for a while?
