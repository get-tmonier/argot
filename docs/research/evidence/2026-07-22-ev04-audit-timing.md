# EV-04 — released-artifact audit timing receipt

**Date:** 2026-07-22
**Status:** measured; not a public performance claim
**Issue:** #170
**Raw receipt:** [2026-07-22-ev04-audit-timing-raw.md](2026-07-22-ev04-audit-timing-raw.md)

## Scope and method

This receipt measures `argot audit --commits 10 --format json` against two
ordinary pinned upstream repositories. It is deliberately a narrow audit
measurement, not a claim about `check`, all repositories, other platforms, or
larger history windows.

The executable was the published release asset, not a source build:

- release: `v0.2.103`, published 2026-07-22T20:46:55Z; release target
  `8b0609bb0d7fc19c4b10ccaf2437860f6ae6e5ec`
- asset: `argot-aarch64-apple-darwin.tar.gz`
- release asset SHA-256 and locally verified archive SHA-256:
  `55a6d5191e71d3d23ea407d3e6dda9c6bd72e8e3bd56ef309fcfd928634c4816`
- extracted executable reports `argot 0.2.103`

Each invocation used isolated `HOME`, `XDG_CACHE_HOME`, and `TMPDIR`; emitted
`ARGOT_TIMING=1` phase data; and was wrapped in `/usr/bin/time -l`. All twelve
qualifying runs exited 0. `cold` here means an empty Argot embedding cache with
the already-fetched model copied into the isolated cache. This makes the
repeatable repository-analysis cost measurable without treating a one-time
network transfer as analysis. `warm` uses a cache prewarmed by one unreported
artifact-only audit of the same repository. `offline` is a copy of that warm
cache with `ARGOT_OFFLINE=1`.

## Repositories

| repository | pinned revision | revision timestamp | 10-commit audit base |
|---|---|---|---|
| `pallets/click` | `cfa01eeb7894a408af70b29d28c0b24f8680f9fb` | 2026-07-20T10:43:21-07:00 | `94c191ca6c95…` |
| `psf/requests` | `69f84847045bef7a849cc994a26fe7ba8a169e95` | 2026-07-20T12:41:16-06:00 | `6f66281a1d6326b1b9c4ac09ca30de0fc4e6ef43` |

The exact clone URLs, revisions, command template, and timing-line receipt
fields are preserved in the raw receipt.

## Environment

- macOS 26.5.1 (25F80), Darwin 25.5.0, arm64
- Mac15,6; 11 logical / 11 physical CPUs; 19,327,352,832 bytes memory
- no HTTP proxy configured by `scutil --proxy`
- release-download probe: GitHub remote `185.199.111.133`, connect 0.136348 s,
  total 2.854473 s
- model: `jina-embeddings-v2-base-code-Q4_K_M.gguf`, 109,451,616 bytes after
  the artifact fetched it into an otherwise empty isolated cache

The isolated first-use probe printed the model-download notice, then recorded
`calibrate: embedder load: 67.76s`, `audit: fit (total): 83.44s`, and
`audit: check (total): 1.83s` on click. It is intentionally not included in
the table: the probe was not wrapped in `/usr/bin/time -l`, so it has no
wall-time or peak-memory receipt. It establishes the separated model-fetch
path rather than supplying a partial timing row.

## Results

Times are seconds. Peak RSS is the `maximum resident set size` reported by
macOS `/usr/bin/time -l`, converted from bytes to MiB (base 2). `fit` and
`check` are the CLI's `audit: ... (total)` phases; remaining wall time covers
worktree setup, attribution, process startup, and measurement overhead.

| repository | cache case | run | wall | fit | check | peak RSS (MiB) |
|---|---|---:|---:|---:|---:|---:|
| click | cold | 1 | 17.92 | 15.97 | 1.81 | 601.5 |
| click | cold | 2 | 17.89 | 15.88 | 1.92 | 615.8 |
| click | warm | 1 | 3.93 | 2.89 | 0.95 | 253.7 |
| click | warm | 2 | 4.03 | 2.99 | 0.95 | 253.2 |
| click | offline | 1 | 3.94 | 2.90 | 0.95 | 254.2 |
| click | offline | 2 | 3.98 | 2.93 | 0.95 | 252.3 |
| requests | cold | 1 | 9.05 | 8.30 | 0.05 | 477.3 |
| requests | cold | 2 | 8.44 | 8.30 | 0.05 | 422.8 |
| requests | warm | 1 | 2.23 | 2.10 | 0.05 | 238.3 |
| requests | warm | 2 | 2.25 | 2.11 | 0.05 | 238.2 |
| requests | offline | 1 | 2.22 | 2.09 | 0.05 | 237.8 |
| requests | offline | 2 | 2.21 | 2.08 | 0.05 | 237.4 |

## Reconciliation and limits

For every row, the recorded phase totals fit within wall time. The raw receipt
also retains worktree, semantic-index, and attribution phase lines, so a
future timing change can locate the residual rather than treating it as an
unexplained total. The second run of every case is an independent process;
the two warm/offline rows are not repeated readings from a live process.

The warm/offline phase lines show all 469 click functions and all 216 requests
functions reused from the embedding cache, whereas cold lines show zero cache
reuse. Offline outputs and timings were materially the same as warm outputs
under `ARGOT_OFFLINE=1`; this is evidence only for these already-warmed,
model-present cache states.

No product copy, benchmark manifest, threshold, or performance target changes
in this issue. These measurements do not justify a general speed promise.
