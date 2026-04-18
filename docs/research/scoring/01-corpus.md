# Scoring Benchmark — Corpus (Phase 2 Pin)

> Living document. Updated as each bucket is verified.

**Scope:** 9 repositories (1 self + 2 per external bucket × 4 buckets) used
to answer "how does AUC scale with dataset size?" See
[`DESIGN.md`](DESIGN.md) §Phase 2 for the corpus rationale.

**Bucket classification:** repos are assigned to buckets by **extracted record
count**, not reputation. Each bucket pairs one TS and one Py repo of similar
scale so the stratified downsample stays balanced (each repo ≥ 40% of target).

**Reproduce the extracts:**

```bash
# Per row below:
git clone <url> .argot/research/repos/<slug>
git -C .argot/research/repos/<slug> checkout <sha>
uv run --package argot-engine python -m argot.extract \
    .argot/research/repos/<slug> \
    --out .argot/research/datasets/<slug>.jsonl \
    --repo-name <slug>
```

## Buckets

| bucket  | slug     | url                                        | sha (pinned)                               | records | language |
|:--------|:---------|:-------------------------------------------|:-------------------------------------------|--------:|:---------|
| micro   | argot    | https://github.com/get-tmonier/argot       | fda874cee3d77563da347d8497ce330300bd8c0c   |     243 | ts+py    |
| small   | TBD      | TBD                                        | TBD                                        |     TBD | ts       |
| small   | ruff     | https://github.com/astral-sh/ruff          | 172ac2c9a27040e4a60726f82cabed6166af094a   |   3,343 | py       |
| medium  | vite     | https://github.com/vitejs/vite             | 0deadcde614fee4fe2a5ccc9f5321dc30bfcca2f   |   8,252 | ts       |
| medium  | click    | https://github.com/pallets/click           | 04ef3a6f473deb2499721a8d11f92a7d2c0912f2   |   6,334 | py       |
| large   | effect   | https://github.com/Effect-TS/effect        | b56a6ec05688c1461574b54a8044a849e7fe639c   |  21,693 | ts       |
| large   | pydantic | https://github.com/pydantic/pydantic       | a6bf50b721c2dd1ed609c8bb402076e8ec0c43f3   |  27,787 | py       |
| xlarge  | vscode   | https://github.com/microsoft/vscode        | TBD                                        |     TBD | ts       |
| xlarge  | django   | https://github.com/django/django           | 1b0d46f715849de53563aaf6912b4ded7d61641d   | 174,877 | py       |

## Bucket targets and composition

| bucket | target records (downsample) | repos             | notes                                               |
|:-------|----------------------------:|:------------------|:----------------------------------------------------|
| micro  |                         250 | argot             | single-repo; skipped in bench                       |
| small  |                       3,000 | TBD + ruff        | TS slot TBD; target provisional on ruff size (3,343)|
| medium |                       7,000 | vite + click      |                                                     |
| large  |                      20,000 | effect + pydantic |                                                     |
| xlarge |                      60,000 | vscode + django   | target provisional; finalise after vscode extract   |

## Dropped repos

These repos were extracted but removed from the corpus after the Phase 2 audit:

| slug                 | records | reason                                                                                         |
|:---------------------|--------:|:-----------------------------------------------------------------------------------------------|
| zod                  |  14,715 | Medium-scale (14k records) — too large for the small target (3k); no clean bucket without a matching py repo of similar size. No longer used. |
| typescript-compiler  |  35,246 | 94% of records are JS test fixtures and baselines from the TypeScript compiler's `tests/` tree, not idiomatic TS application code. Dropped in favour of vscode. |

## Swap-out log

| task | bucket | candidate           | reason                                                                            | replacement         |
|-----:|:-------|:--------------------|:----------------------------------------------------------------------------------|:--------------------|
|    3 | medium | vigie               | 1,194 records < 2,400 floor                                                       | vite v2.0.0         |
|    5 | xlarge | tsgo                | microsoft/tsgo not found; typescript-go is Go, not extractable by argot           | microsoft/TypeScript (later dropped) |
|    7 | xlarge | typescript-compiler | 94% JS fixtures; 35k records → only 17% share in 60k sample, below 40% floor     | microsoft/vscode    |
|    7 | small  | zod                 | 14,715 records — medium-scale, not small; corpus audit reclassification           | TBD small TS repo   |
