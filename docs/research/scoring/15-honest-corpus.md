# Phase 7.0 — Honest Corpus

**Branch**: `research/phase-7-honest-eval`
**Status**: in progress — SHAs pinned at clone time; counts filled in post-extract

## Design

Same-language sub-corpora per bucket. Each bucket has a Python pair and a
TypeScript pair benchmarked **independently** (no TS+Py mixing).

Selection criteria:
- **Same domain** where possible (controls for domain-specific vocabulary).
- **Strong style contrast** on ≥ 3 axes from the Phase 7 style-axis reference
  (`DESIGN-phase-7.md` §Style-axis reference).

## Repos

| bucket | lang | repo A (URL, SHA)                            | repo B (URL, SHA)                            |
|:-------|:-----|:---------------------------------------------|:---------------------------------------------|
| small  | py   | https://github.com/encode/httpx @ <SHA>      | https://github.com/psf/requests @ <SHA>      |
| small  | ts   | https://github.com/sindresorhus/ky @ <SHA>   | https://github.com/colinhacks/zod @ c7805073fef5b6b8857307c3d4b3597a70613bc2 |
| medium | py   | https://github.com/tiangolo/fastapi @ <SHA>  | https://github.com/pallets/flask @ <SHA>     |
| medium | ts   | https://github.com/vitejs/vite @ <SHA>       | https://github.com/rollup/rollup @ <SHA>     |
| large  | py   | https://github.com/pydantic/pydantic @ <SHA> | https://github.com/django/django @ <SHA>     |
| large  | ts   | https://github.com/Effect-TS/effect @ <SHA>  | https://github.com/angular/angular @ <SHA>   |

**Pin SHAs to whatever `HEAD` is at clone time** (documented below per repo).
Do not hunt for a "nice" commit; reproducibility is what matters.

## Extracted record counts (post-extract)

Filled in after Task 11.

| repo          | records |
|:--------------|--------:|
| httpx         |       ? |
| requests      |       ? |
| ky            |       ? |
| zod           |   14575 |
| fastapi       |       ? |
| flask         |       ? |
| vite          |       ? |
| rollup        |       ? |
| pydantic      |       ? |
| django        |       ? |
| effect        |       ? |
| angular       |       ? |

## Bucket composition (post-downsample)

Each bucket concatenates its two same-language repos and downsamples to the
target count. Minimum share per sub-corpus after downsample: ≥ 40%.

| bucket | lang | target | actual (home + foreign, share) |
|:-------|:-----|-------:|:-------------------------------|
| small  | py   |   3000 | ?                              |
| small  | ts   |   3000 | ?                              |
| medium | py   |   7000 | ?                              |
| medium | ts   |   7000 | ?                              |
| large  | py   |  20000 | ?                              |
| large  | ts   |  20000 | ?                              |

If targets are unreachable (e.g. both chalk + ky land below 3000), rename the
bucket to its actual size rather than force-fit.

## Mutations

See `DESIGN-phase-7.md` §Feasible mutation set. Four mutations:
`case_swap`, `debug_inject`, `error_flip`, `quote_flip`.

Sample mutated records (filled in Task 14 after the sanity run).

## Harness usage

```bash
just research-honest-benchmark
# → runs the six (bucket × lang) combinations with the chosen encoder
#   writes to .argot/research/results-honest.jsonl
```

## Swap-out log

- **chalk** dropped: 130 TS records — too few for a viable small-ts bucket. Replaced with **axios** (first attempt).
- **axios** also dropped: 488 TS records (94% JS) — same problem as chalk. Replaced with **colinhacks/zod** (pure TypeScript, schema-validation library, strong style contrast with ky).
