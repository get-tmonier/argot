# EV-04 raw timing receipts — 2026-07-22

This file preserves the exact timing-line and `/usr/bin/time -l` values from
the twelve qualifying calls described in
[the evidence record](2026-07-22-ev04-audit-timing.md). Values were copied
without aggregation from each invocation's stderr. All calls exited 0.

## Command and provenance

```text
<released-v0.2.103-argot> audit --commits 10 --format json --repo <pinned-clone>
```

The command was invoked under `ARGOT_TIMING=1`, isolated `HOME`,
`XDG_CACHE_HOME`, and `TMPDIR`, and `/usr/bin/time -l`; offline calls also set
`ARGOT_OFFLINE=1`. Clone URLs and pinned revisions:

```text
https://github.com/pallets/click.git    cfa01eeb7894a408af70b29d28c0b24f8680f9fb
https://github.com/psf/requests.git     69f84847045bef7a849cc994a26fe7ba8a169e95
```

## Exact timing-line receipts

| receipt | worktree | semantic index | fit | check | wall (`time -l`) | peak RSS bytes |
|---|---:|---|---:|---:|---:|---:|
| cold-click-1 | 0.05s | 469 fns; 0 cache-reused; 12.99s | 15.97s | 1.81s | 17.92 | 630669312 |
| cold-click-2 | 0.04s | 469 fns; 0 cache-reused; 12.93s | 15.88s | 1.92s | 17.89 | 645677056 |
| warm-click-1 | 0.04s | 469 fns; 469 cache-reused; 0.00s | 2.89s | 0.95s | 3.93 | 266059776 |
| warm-click-2 | 0.04s | 469 fns; 469 cache-reused; 0.01s | 2.99s | 0.95s | 4.03 | 265486336 |
| offline-click-1 | 0.04s | 469 fns; 469 cache-reused; 0.00s | 2.90s | 0.95s | 3.94 | 266518528 |
| offline-click-2 | 0.04s | 469 fns; 469 cache-reused; 0.00s | 2.93s | 0.95s | 3.98 | 264585216 |
| cold-requests-1 | 0.59s | 216 fns; 0 cache-reused; 6.10s | 8.30s | 0.05s | 9.05 | 500482048 |
| cold-requests-2 | 0.04s | 216 fns; 0 cache-reused; 6.11s | 8.30s | 0.05s | 8.44 | 443383808 |
| warm-requests-1 | 0.04s | 216 fns; 216 cache-reused; 0.00s | 2.10s | 0.05s | 2.23 | 249839616 |
| warm-requests-2 | 0.04s | 216 fns; 216 cache-reused; 0.00s | 2.11s | 0.05s | 2.25 | 249774080 |
| offline-requests-1 | 0.04s | 216 fns; 216 cache-reused; 0.00s | 2.09s | 0.05s | 2.22 | 249348096 |
| offline-requests-2 | 0.04s | 216 fns; 216 cache-reused; 0.00s | 2.08s | 0.05s | 2.21 | 248954880 |

Each receipt also emitted `audit: attribution: 0.00s`. The source stderr lines
were respectively `audit: worktree add + seed`, `calibrate[python]: semantic
embed`, `audit: fit (total)`, `audit: check (total)`, and the macOS
`maximum resident set size` field. The raw command JSON confirmed requested
and effective windows of 10 and the bases
`94c191ca6c9598865fc5672b85cf138845b337d5` (click) and
`6f66281a1d6326b1b9c4ac09ca30de0fc4e6ef43` (requests); every invocation
returned exit status 0.

## One-time model-fetch separation receipt

Before qualifying runs, a distinct fully isolated artifact-only click probe
with no model cache emitted:

```text
argot: downloading jina-embeddings-v2-base-code (~100 MB, one-time) ...
argot: semantic model ready (.../jina-embeddings-v2-base-code-Q4_K_M.gguf)
[timing] calibrate: embedder load: 67.76s
[timing] calibrate[python]: semantic embed (469 fns, 0 prior-reused, 0 cache-reused): 12.93s
[timing] audit: fit (total): 83.44s
[timing] audit: check (total): 1.83s
```

The resulting model file was 109451616 bytes. The probe intentionally has no
wall/RSS entry because it was used to isolate download/load from the qualifying
analysis measurements and was not wrapped by `/usr/bin/time -l`.
